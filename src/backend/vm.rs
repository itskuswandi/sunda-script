use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    backend::bytecode::{Chunk, OperationCode, Value},
    error::{ScriptError, ScriptResult},
};

pub struct VM {
    stack: Vec<Value>,
    globals: HashMap<String, Value>,
    call_stack: Vec<(usize, usize)>,
}

impl VM {
    pub fn new() -> Self {
        VM {
            stack: Vec::new(),
            globals: HashMap::new(),
            call_stack: Vec::new(),
        }
    }

    pub fn run(&mut self, chunk: &Chunk) -> ScriptResult<()> {
        self.run_loop(chunk, 0, 0, 0)?;
        Ok(())
    }

    pub fn execute_function_sync(
        &mut self,
        chunk: &Chunk,
        function_address: usize,
        args_len: usize,
    ) -> Result<Value, String> {
        let target_depth = self.call_stack.len();
        let new_frame_pointer = self.stack.len() - args_len;

        match self.run_loop(chunk, function_address, new_frame_pointer, target_depth) {
            Ok(value) => Ok(value),
            Err(error) => match error {
                ScriptError::RuntimeError { message, .. } => Err(message),
                _ => Err("Lepat sistem: Gagal ngajalankeun pancen ti jero metode.".to_string()),
            },
        }
    }

    fn run_loop(
        &mut self,
        chunk: &Chunk,
        start_ip: usize,
        start_frame: usize,
        target_call_depth: usize,
    ) -> ScriptResult<Value> {
        let mut ip = start_ip;
        let mut frame_pointer = start_frame;

        while ip < chunk.code.len() {
            let instruction = &chunk.code[ip];
            let current_line = chunk.lines[ip];

            ip += 1;

            match instruction {
                OperationCode::Constant(index) => {
                    let value = chunk.constants[*index].clone();
                    self.stack.push(value);
                }

                OperationCode::Pop => {
                    self.pop_stack(current_line)?;
                }

                OperationCode::Add => {
                    let right = self.pop_stack(current_line)?;
                    let left = self.pop_stack(current_line)?;

                    match (left, right) {
                        (Value::Integer(l), Value::Integer(r)) => {
                            self.stack.push(Value::Integer(l + r))
                        }
                        (Value::Float(l), Value::Float(r)) => self.stack.push(Value::Float(l + r)),
                        (Value::Integer(l), Value::Float(r)) => {
                            self.stack.push(Value::Float(l as f64 + r))
                        }
                        (Value::Float(l), Value::Integer(r)) => {
                            self.stack.push(Value::Float(l + r as f64))
                        }

                        (Value::String(l), r) => {
                            let right_string = self.format_value(&r);
                            self.stack
                                .push(Value::String(Rc::new(format!("{}{}", l, right_string))));
                        }
                        (l, Value::String(r)) => {
                            let left_string = self.format_value(&l);
                            self.stack
                                .push(Value::String(Rc::new(format!("{}{}", left_string, r))));
                        }
                        _ => {
                            return Err(self.error(
                                current_line,
                                "Operasi panambihan mung kanggo angka sareng teks.",
                            ));
                        }
                    }
                }

                OperationCode::Subtract => {
                    let right = self.pop_stack(current_line)?;
                    let left = self.pop_stack(current_line)?;

                    match (left, right) {
                        (Value::Integer(l), Value::Integer(r)) => {
                            self.stack.push(Value::Integer(l - r))
                        }
                        (Value::Float(l), Value::Float(r)) => self.stack.push(Value::Float(l - r)),
                        (Value::Integer(l), Value::Float(r)) => {
                            self.stack.push(Value::Float(l as f64 - r))
                        }
                        (Value::Float(l), Value::Integer(r)) => {
                            self.stack.push(Value::Float(l - r as f64))
                        }
                        _ => {
                            return Err(
                                self.error(current_line, "Operasi pangurangan mung kanggo angka.")
                            );
                        }
                    }
                }

                OperationCode::Multiply => {
                    let right = self.pop_stack(current_line)?;
                    let left = self.pop_stack(current_line)?;

                    match (left, right) {
                        (Value::Integer(l), Value::Integer(r)) => {
                            self.stack.push(Value::Integer(l * r))
                        }
                        (Value::Float(l), Value::Float(r)) => self.stack.push(Value::Float(l * r)),
                        (Value::Integer(l), Value::Float(r)) => {
                            self.stack.push(Value::Float(l as f64 * r))
                        }
                        (Value::Float(l), Value::Integer(r)) => {
                            self.stack.push(Value::Float(l * r as f64))
                        }
                        _ => {
                            return Err(
                                self.error(current_line, "Operasi pangalian mung kanggo angka.")
                            );
                        }
                    }
                }

                OperationCode::Divide => {
                    let right = self.pop_stack(current_line)?;
                    let left = self.pop_stack(current_line)?;

                    match (left, right) {
                        (Value::Integer(l), Value::Integer(r)) => {
                            if r == 0 {
                                return Err(self.error(current_line, "Teu tiasa ngabagi ku enol."));
                            }
                            self.stack.push(Value::Integer(l / r));
                        }
                        (Value::Float(l), Value::Float(r)) => {
                            if r == 0.0 {
                                return Err(self.error(current_line, "Teu tiasa ngabagi ku enol."));
                            }
                            self.stack.push(Value::Float(l / r));
                        }
                        (Value::Integer(l), Value::Float(r)) => {
                            if r == 0.0 {
                                return Err(self.error(current_line, "Teu tiasa ngabagi ku enol."));
                            }
                            self.stack.push(Value::Float(l as f64 / r));
                        }
                        (Value::Float(l), Value::Integer(r)) => {
                            if r == 0 {
                                return Err(self.error(current_line, "Teu tiasa ngabagi ku enol."));
                            }
                            self.stack.push(Value::Float(l / r as f64));
                        }
                        _ => {
                            return Err(
                                self.error(current_line, "Operasi ngabagi mung kanggo angka.")
                            );
                        }
                    }
                }

                OperationCode::Modulo => {
                    let right = self.pop_stack(current_line)?;
                    let left = self.pop_stack(current_line)?;

                    match (left, right) {
                        (Value::Integer(l), Value::Integer(r)) => {
                            if r == 0 {
                                return Err(self.error(
                                    current_line,
                                    "Teu tiasa ngitung sesa ngabagi ku enol.",
                                ));
                            }
                            self.stack.push(Value::Integer(l % r));
                        }
                        (Value::Float(l), Value::Float(r)) => {
                            if r == 0.0 {
                                return Err(self.error(
                                    current_line,
                                    "Teu tiasa ngitung sesa ngabagi ku enol.",
                                ));
                            }
                            self.stack.push(Value::Float(l % r));
                        }
                        (Value::Integer(l), Value::Float(r)) => {
                            if r == 0.0 {
                                return Err(self.error(
                                    current_line,
                                    "Teu tiasa ngitung sesa ngabagi ku enol.",
                                ));
                            }
                            self.stack.push(Value::Float(l as f64 % r));
                        }
                        (Value::Float(l), Value::Integer(r)) => {
                            if r == 0 {
                                return Err(self.error(
                                    current_line,
                                    "Teu tiasa ngitung sesa ngabagi ku enol.",
                                ));
                            }
                            self.stack.push(Value::Float(l % r as f64));
                        }
                        _ => {
                            return Err(
                                self.error(current_line, "Operasi sesa ngabagi mung kanggo angka.")
                            );
                        }
                    }
                }

                OperationCode::Negate => {
                    let value = self.pop_stack(current_line)?;
                    match value {
                        Value::Integer(i) => self.stack.push(Value::Integer(-i)),
                        Value::Float(f) => self.stack.push(Value::Float(-f)),
                        _ => {
                            return Err(
                                self.error(current_line, "Tanda minus mung tiasa kanggo angka.")
                            );
                        }
                    }
                }

                OperationCode::Not => {
                    let value = self.pop_stack(current_line)?;
                    match value {
                        Value::Boolean(b) => self.stack.push(Value::Boolean(!b)),
                        _ => {
                            return Err(self.error(
                                current_line,
                                "Tanda seru mung tiasa kanggo nilai leres atanapi lepat.",
                            ));
                        }
                    }
                }

                OperationCode::Equal => {
                    let right = self.pop_stack(current_line)?;
                    let left = self.pop_stack(current_line)?;
                    self.stack.push(Value::Boolean(left == right));
                }

                OperationCode::GreaterThan => {
                    let right = self.pop_stack(current_line)?;
                    let left = self.pop_stack(current_line)?;

                    match (left, right) {
                        (Value::Integer(l), Value::Integer(r)) => {
                            self.stack.push(Value::Boolean(l > r))
                        }
                        (Value::Float(l), Value::Float(r)) => {
                            self.stack.push(Value::Boolean(l > r))
                        }
                        (Value::Integer(l), Value::Float(r)) => {
                            self.stack.push(Value::Boolean((l as f64) > r))
                        }
                        (Value::Float(l), Value::Integer(r)) => {
                            self.stack.push(Value::Boolean(l > r as f64))
                        }
                        _ => {
                            return Err(self.error(current_line, "Babandingan mung kanggo angka."));
                        }
                    }
                }

                OperationCode::LessThan => {
                    let right = self.pop_stack(current_line)?;
                    let left = self.pop_stack(current_line)?;

                    match (left, right) {
                        (Value::Integer(l), Value::Integer(r)) => {
                            self.stack.push(Value::Boolean(l < r))
                        }
                        (Value::Float(l), Value::Float(r)) => {
                            self.stack.push(Value::Boolean(l < r))
                        }
                        (Value::Integer(l), Value::Float(r)) => {
                            self.stack.push(Value::Boolean((l as f64) < r))
                        }
                        (Value::Float(l), Value::Integer(r)) => {
                            self.stack.push(Value::Boolean(l < r as f64))
                        }
                        _ => {
                            return Err(self.error(current_line, "Babandingan mung kanggo angka."));
                        }
                    }
                }

                OperationCode::SetGlobal(index) => {
                    let name = match &chunk.constants[*index] {
                        Value::String(s) => s.to_string(),
                        _ => {
                            return Err(self
                                .error(current_line, "Nami variabel global kedah mangrupa teks."));
                        }
                    };

                    let value = self
                        .stack
                        .last()
                        .ok_or_else(|| {
                            self.error(current_line, "Tumpukan kosong nalika nyimpen variabel.")
                        })?
                        .clone();

                    self.globals.insert(name, value);
                }

                OperationCode::GetGlobal(index) => {
                    let rc_name = match &chunk.constants[*index] {
                        Value::String(s) => s,
                        _ => {
                            return Err(self
                                .error(current_line, "Nami variabel global kedah mangrupa teks."));
                        }
                    };

                    if let Some(value) = self.globals.get(rc_name.as_str()) {
                        self.stack.push(value.clone());
                    } else {
                        return Err(self.error(
                            current_line,
                            &format!("Variabel global '{}' teu acan didamel.", rc_name),
                        ));
                    }
                }

                OperationCode::SetLocal(index) => {
                    let value = self
                        .stack
                        .last()
                        .ok_or_else(|| {
                            self.error(current_line, "Tumpukan kosong nalika nyimpen variabel.")
                        })?
                        .clone();

                    self.stack[frame_pointer + *index] = value;
                }

                OperationCode::GetLocal(index) => {
                    let value = self.stack[frame_pointer + *index].clone();
                    self.stack.push(value);
                }

                OperationCode::BuildArray(element_count) => {
                    let start_index = self.stack.len() - *element_count;
                    let elements: Vec<Value> = self.stack.drain(start_index..).collect();

                    self.stack
                        .push(Value::Array(Rc::new(RefCell::new(elements))));
                }

                OperationCode::GetIndex => {
                    let index_value = self.pop_stack(current_line)?;
                    let array_value = self.pop_stack(current_line)?;

                    if let Value::Array(array) = array_value {
                        if let Value::Integer(i) = index_value {
                            let elements = array.borrow();
                            if i < 0 || i as usize >= elements.len() {
                                return Err(
                                    self.error(current_line, "Indeks daptar di luar wates.")
                                );
                            }
                            self.stack.push(elements[i as usize].clone());
                        } else {
                            return Err(self.error(
                                current_line,
                                "Indeks daptar kedah mangrupa angka buleud.",
                            ));
                        }
                    } else {
                        return Err(self.error(
                            current_line,
                            "Mung daptar anu tiasa dicandak indeksna ku [].",
                        ));
                    }
                }

                OperationCode::SetIndex => {
                    let new_value = self.pop_stack(current_line)?;
                    let index_value = self.pop_stack(current_line)?;
                    let array_value = self.pop_stack(current_line)?;

                    if let Value::Array(array) = array_value {
                        if let Value::Integer(i) = index_value {
                            let mut elements = array.borrow_mut();

                            if i < 0 || i as usize >= elements.len() {
                                return Err(
                                    self.error(current_line, "Indeks daptar di luar wates.")
                                );
                            }

                            elements[i as usize] = new_value.clone();
                            self.stack.push(new_value);
                        } else {
                            return Err(self.error(
                                current_line,
                                "Indeks daptar kedah mangrupa angka buleud.",
                            ));
                        }
                    } else {
                        return Err(self.error(
                            current_line,
                            "Mung daptar anu tiasa dirobih indeksna ku [].",
                        ));
                    }
                }

                OperationCode::BuildObject(properties_count) => {
                    let mut map = HashMap::new();

                    for _ in 0..*properties_count {
                        let value = self.pop_stack(current_line)?;
                        let key_value = self.pop_stack(current_line)?;

                        if let Value::String(key) = key_value {
                            map.insert(key.to_string(), value);
                        }
                    }

                    self.stack.push(Value::Object(Rc::new(RefCell::new(map))));
                }

                OperationCode::GetProperty(name_index) => {
                    let rc_name = match &chunk.constants[*name_index] {
                        Value::String(s) => s,
                        _ => {
                            return Err(self.error(current_line, "Nami properti teu sah."));
                        }
                    };

                    let object_value = self.pop_stack(current_line)?;

                    match object_value {
                        Value::Integer(_) => {
                            let bound_method = match rc_name.as_str() {
                                "janten_desimal" => Some(Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    integer_to_float,
                                )),
                                "janten_teks" => Some(Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    integer_to_string,
                                )),
                                "mutlak" => Some(Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    integer_abs,
                                )),
                                "watesan" => Some(Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    integer_clamp,
                                )),
                                _ => None,
                            };

                            if let Some(method) = bound_method {
                                self.stack.push(method);
                            } else {
                                return Err(self.error(
                                    current_line,
                                    &format!("Angka buleud teu ngagaduhan metode '{}'.", rc_name),
                                ));
                            }
                        }

                        Value::Float(_) => {
                            let bound_method = match rc_name.as_str() {
                                "buleudkeun" => Some(Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    float_round,
                                )),
                                "janten_teks" => Some(Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    float_to_string,
                                )),
                                "ka_handap" => Some(Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    float_floor,
                                )),
                                "ka_luhur" => Some(Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    float_ceil,
                                )),
                                "mutlak" => Some(Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    float_abs,
                                )),
                                "watesan" => Some(Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    float_clamp,
                                )),
                                _ => None,
                            };

                            if let Some(method) = bound_method {
                                self.stack.push(method);
                            } else {
                                return Err(self.error(
                                    current_line,
                                    &format!("Angka pecahan teu ngagaduhan metode '{}'.", rc_name),
                                ));
                            }
                        }

                        Value::String(_) => {
                            let bound_method = match rc_name.as_str() {
                                "aksara_ageung" => Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    string_uppercase,
                                ),
                                "aksara_alit" => Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    string_lowercase,
                                ),
                                "balikkeun" => Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    string_reverse,
                                ),
                                "gentos" => Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    string_replace,
                                ),
                                "hurup_ka" => Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    string_char_at,
                                ),
                                "janten_angka" => Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    string_to_integer,
                                ),
                                "janten_desimal" => Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    string_to_float,
                                ),
                                "ngandung" => Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    string_contains,
                                ),
                                "panjang" => Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    string_length,
                                ),
                                "pisahkeun" => Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    string_split,
                                ),
                                "posisi_ka" => Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    string_index_of,
                                ),
                                "potong" => Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    string_slice,
                                ),
                                "rapikeun" => {
                                    Value::NativeMethod(Box::new(object_value.clone()), string_trim)
                                }
                                _ => {
                                    return Err(self.error(
                                        current_line,
                                        &format!("Teks teu ngagaduhan metode '{}'.", rc_name),
                                    ));
                                }
                            };
                            self.stack.push(bound_method);
                        }

                        Value::Array(_) => {
                            let bound_method = match rc_name.as_str() {
                                "balikkeun" => Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    array_reverse,
                                ),
                                "candak_payun" => {
                                    Value::NativeMethod(Box::new(object_value.clone()), array_shift)
                                }
                                "candak_pengker" => {
                                    Value::NativeMethod(Box::new(object_value.clone()), array_pop)
                                }
                                "gabungkeun" => {
                                    Value::NativeMethod(Box::new(object_value.clone()), array_join)
                                }
                                "milari" => {
                                    Value::NativeHOF(Box::new(object_value.clone()), array_find)
                                }
                                "panjang" => Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    array_length,
                                ),
                                "petakeun" => {
                                    Value::NativeHOF(Box::new(object_value.clone()), array_map)
                                }
                                "posisi_ka" => Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    array_index_of,
                                ),
                                "potong" => {
                                    Value::NativeMethod(Box::new(object_value.clone()), array_slice)
                                }
                                "saring" => {
                                    Value::NativeHOF(Box::new(object_value.clone()), array_filter)
                                }
                                "susun" => {
                                    Value::NativeMethod(Box::new(object_value.clone()), array_sort)
                                }
                                "tambih_payun" => Value::NativeMethod(
                                    Box::new(object_value.clone()),
                                    array_unshift,
                                ),
                                "tambih_pengker" => {
                                    Value::NativeMethod(Box::new(object_value.clone()), array_push)
                                }
                                "unggal" => {
                                    Value::NativeHOF(Box::new(object_value.clone()), array_foreach)
                                }
                                _ => {
                                    return Err(self.error(
                                        current_line,
                                        &format!("Daptar teu ngagaduhan metode '{}'.", rc_name),
                                    ));
                                }
                            };
                            self.stack.push(bound_method);
                        }

                        Value::Object(object) => {
                            let map = object.borrow();

                            if let Some(value) = map.get(rc_name.as_str()) {
                                self.stack.push(value.clone());
                            } else {
                                let bound_method = match rc_name.as_str() {
                                    "eusi" => Some(Value::NativeMethod(
                                        Box::new(Value::Object(object.clone())),
                                        object_values,
                                    )),
                                    "jumlah" => Some(Value::NativeMethod(
                                        Box::new(Value::Object(object.clone())),
                                        object_size,
                                    )),
                                    "konci" => Some(Value::NativeMethod(
                                        Box::new(Value::Object(object.clone())),
                                        object_keys,
                                    )),
                                    "ngagaduhan" => Some(Value::NativeMethod(
                                        Box::new(Value::Object(object.clone())),
                                        object_has,
                                    )),
                                    "piceun" => Some(Value::NativeMethod(
                                        Box::new(Value::Object(object.clone())),
                                        object_remove,
                                    )),
                                    _ => None,
                                };

                                if let Some(method) = bound_method {
                                    self.stack.push(method);
                                } else {
                                    return Err(self.error(current_line, &format!("Properti atanapi metode '{}' teu kapendak dina objek ieu.", rc_name)));
                                }
                            }
                        }

                        _ => {
                            return Err(self.error(
                                current_line,
                                "Mung objek, teks, atanapi daptar anu ngagaduhan properti/metode.",
                            ));
                        }
                    }
                }

                OperationCode::SetProperty(name_index) => {
                    let property_name = match &chunk.constants[*name_index] {
                        Value::String(s) => s.to_string(),
                        _ => {
                            return Err(self.error(current_line, "Nami properti teu sah."));
                        }
                    };

                    let new_value = self.pop_stack(current_line)?;
                    let object_value = self.pop_stack(current_line)?;

                    if let Value::Object(object) = object_value {
                        let mut map = object.borrow_mut();
                        map.insert(property_name, new_value.clone());
                        self.stack.push(new_value);
                    } else {
                        return Err(
                            self.error(current_line, "Mung objek anu tiasa dirobih propertina.")
                        );
                    }
                }

                OperationCode::JumpIfFalse(target) => {
                    let condition = self.stack.last().ok_or_else(|| {
                        self.error(current_line, "Tumpukan kosong nalika maca kondisi.")
                    })?;

                    match condition {
                        Value::Boolean(b) => {
                            if !b {
                                ip = *target
                            }
                        }
                        _ => {
                            return Err(self.error(
                                current_line,
                                "Kondisi kedah mangrupa 'leres' atanapi 'lepat'.",
                            ));
                        }
                    }
                }

                OperationCode::Jump(target) => ip = *target,

                OperationCode::Call(arity) => {
                    let argument_count = *arity;

                    if self.stack.len() < argument_count + 1 {
                        return Err(self.error(
                            current_line,
                            "Tumpukan kirang data nalika ngajalankeun pancen.",
                        ));
                    }

                    let function_index = self.stack.len() - 1 - argument_count;
                    let function_value = self.stack[function_index].clone();

                    match function_value {
                        Value::Function(function) => {
                            if function.arity != argument_count {
                                return Err(self.error(
                                    current_line,
                                    &format!(
                                        "Pancen '{}' peryogi {} argumen, tapi dipasihan {}.",
                                        function.name, function.arity, arity
                                    ),
                                ));
                            }

                            self.call_stack.push((ip, frame_pointer));
                            frame_pointer = self.stack.len() - argument_count;
                            ip = function.address;
                        }

                        Value::NativeMethod(receiver, native_fn) => {
                            let arguments = &self.stack[self.stack.len() - argument_count..];

                            match native_fn(&receiver, arguments) {
                                Ok(result) => {
                                    self.stack.truncate(self.stack.len() - argument_count - 1);
                                    self.stack.push(result);
                                }
                                Err(error_message) => {
                                    return Err(self.error(current_line, &error_message));
                                }
                            }
                        }

                        Value::NativeHOF(receiver, hof_fun) => {
                            let arguments =
                                self.stack[self.stack.len() - argument_count..].to_vec();
                            self.stack.truncate(self.stack.len() - argument_count - 1);

                            match hof_fun(self, chunk, &receiver, &arguments) {
                                Ok(result) => self.stack.push(result),
                                Err(error_message) => {
                                    return Err(self.error(current_line, &error_message));
                                }
                            }
                        }

                        _ => {
                            return Err(self.error(current_line, "Anu dijalankeun sanes pancen."));
                        }
                    }
                }

                OperationCode::Return => {
                    let return_value = self.pop_stack(current_line)?;

                    if self.call_stack.len() == target_call_depth {
                        self.stack.truncate(frame_pointer - 1);
                        return Ok(return_value);
                    }

                    if let Some((return_address, previous_frame_pointer)) = self.call_stack.pop() {
                        ip = return_address;

                        self.stack.truncate(frame_pointer - 1);
                        self.stack.push(return_value);

                        frame_pointer = previous_frame_pointer
                    } else {
                        return Err(self.error(current_line, "Teu tiasa 'wangsul' di luar pancen."));
                    }
                }

                OperationCode::Print => {
                    let value = self.pop_stack(current_line)?;
                    println!("{}", self.format_value(&value));
                }
            }
        }

        Ok(Value::Null)
    }

    fn format_value(&self, value: &Value) -> String {
        match value {
            Value::Integer(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(s) => s.to_string(),
            Value::Boolean(b) => (if *b { "leres" } else { "lepat" }).to_string(),
            Value::Null => "kosong".to_string(),

            Value::Array(array) => {
                let elements = array.borrow();
                let mut result = String::from("[");

                for (i, value) in elements.iter().enumerate() {
                    if let Value::String(s) = value {
                        result.push_str(&format!("\"{}\"", s));
                    } else {
                        result.push_str(&self.format_value(value));
                    }
                    if i < elements.len() - 1 {
                        result.push_str(", ");
                    }
                }

                result.push(']');
                result
            }

            Value::Object(object) => {
                let map = object.borrow();
                let mut result = String::from("{ ");
                let mut i = 0;
                let len = map.len();

                for (key, value) in map.iter() {
                    result.push_str(&format!("{}: ", key));

                    if let Value::String(s) = value {
                        result.push_str(&format!("\"{}\"", s));
                    } else {
                        result.push_str(&self.format_value(value));
                    }

                    if i < len - 1 {
                        result.push_str(", ");
                    }
                    i += 1;
                }

                result.push_str(" }");
                result
            }

            Value::Function(f) => format!("<pancen {} ({} parameter)>", f.name, f.arity),
            Value::NativeMethod(_, _) => "<metode bawaan>".to_string(),
            Value::NativeHOF(_, _) => "<metode bawaan tingkat luhur>".to_string(),
        }
    }

    fn pop_stack(&mut self, line: usize) -> ScriptResult<Value> {
        self.stack
            .pop()
            .ok_or_else(|| self.error(line, "Tumpukan kosong. Aya lepat dina alur kompilasi."))
    }

    fn error(&self, line: usize, message: &str) -> ScriptError {
        ScriptError::RuntimeError {
            line,
            message: message.to_string(),
        }
    }
}

fn array_filter(
    vm: &mut VM,
    chunk: &Chunk,
    receiver: &Value,
    args: &[Value],
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("Metode 'saring' peryogi 1 argumen pancen.".to_string());
    }

    let (function_address, arity) = match &args[0] {
        Value::Function(function) => (function.address, function.arity),
        _ => return Err("Argumen 'saring' kedah mangrupa pancen.".to_string()),
    };

    if arity != 1 {
        return Err(format!(
            "Pancen keur 'saring' peryogi 1 parameter, tapi dipasihan {}.",
            arity
        ));
    }

    if let Value::Array(array) = receiver {
        let elements = array.borrow().clone();
        let mut filtered_elements = Vec::new();

        for element in elements {
            vm.stack.push(args[0].clone());
            vm.stack.push(element.clone());

            let result = vm.execute_function_sync(chunk, function_address, 1)?;

            match result {
                Value::Boolean(b) => {
                    if b {
                        filtered_elements.push(element.clone());
                    }
                }
                _ => {
                    return Err(
                        "Pancen keur 'saring' kedah mulangkeun nilai 'leres' atanapi 'lepat'."
                            .to_string(),
                    );
                }
            }
        }

        Ok(Value::Array(Rc::new(RefCell::new(filtered_elements))))
    } else {
        Err("Aya lepat sistem: panarima sanes daptar.".to_string())
    }
}

fn array_find(
    vm: &mut VM,
    chunk: &Chunk,
    receiver: &Value,
    args: &[Value],
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("Metode 'milari' peryogi 1 argumen pancen.".to_string());
    }

    let (function_address, arity) = match &args[0] {
        Value::Function(function) => (function.address, function.arity),
        _ => return Err("Argumen 'milari' kedah mangrupa pancen.".to_string()),
    };

    if arity != 1 {
        return Err(format!(
            "Pancen keur 'milari' peryogi 1 parameter, tapi dipasihan {}.",
            arity
        ));
    }

    if let Value::Array(array) = receiver {
        let elements = array.borrow().clone();

        for element in elements {
            vm.stack.push(args[0].clone());
            vm.stack.push(element.clone());

            let result = vm.execute_function_sync(chunk, function_address, 1)?;

            match result {
                Value::Boolean(b) => {
                    if b {
                        return Ok(element.clone());
                    }
                }
                _ => {
                    return Err(
                        "Pancen keur 'milari' kedah mulangkeun nilai 'leres' atanapi 'lepat'."
                            .to_string(),
                    );
                }
            }
        }

        Ok(Value::Null)
    } else {
        Err("Aya lepat sistem: panarima sanes daptar.".to_string())
    }
}

fn array_foreach(
    vm: &mut VM,
    chunk: &Chunk,
    receiver: &Value,
    args: &[Value],
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("Metode 'unggal' peryogi 1 argumen pancen.".to_string());
    }

    let (function_address, arity) = match &args[0] {
        Value::Function(function) => (function.address, function.arity),
        _ => return Err("Argumen 'unggal' kedah mangrupa pancen.".to_string()),
    };

    if arity != 1 {
        return Err(format!(
            "Pancen keur 'unggal' peryogi 1 parameter, tapi dipasihan {}.",
            arity
        ));
    }

    if let Value::Array(array) = receiver {
        let elements = array.borrow().clone();

        for element in elements {
            vm.stack.push(args[0].clone());
            vm.stack.push(element.clone());
            vm.execute_function_sync(chunk, function_address, 1)?;
        }

        Ok(Value::Null)
    } else {
        Err("Aya lepat sistem: panarima sanes daptar.".to_string())
    }
}

fn array_index_of(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("Metode 'posisi_ka' peryogi 1 argumen.".to_string());
    }

    if let Value::Array(array) = receiver {
        let elements = array.borrow();
        let search_value = &args[0];

        for (index, element) in elements.iter().enumerate() {
            if element == search_value {
                return Ok(Value::Integer(index as i64));
            }
        }

        Ok(Value::Integer(-1))
    } else {
        Err("Aya lepat sistem: panarima sanes daptar.".to_string())
    }
}

fn array_join(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("Metode 'gabungkeun' peryogi 1 argumen pamisa.".to_string());
    }

    if let Value::Array(array) = receiver {
        let delimiter = match &args[0] {
            Value::String(s) => s.as_str(),
            _ => return Err("Argumen pamisa kedah mangrupa teks.".to_string()),
        };

        let elements = array.borrow();
        let mut string_elements = Vec::new();

        for element in elements.iter() {
            match element {
                Value::Integer(i) => string_elements.push(i.to_string()),
                Value::Float(f) => string_elements.push(f.to_string()),
                Value::String(s) => string_elements.push(s.to_string()),
                Value::Boolean(b) => string_elements.push(if *b {
                    "leres".to_string()
                } else {
                    "lepat".to_string()
                }),
                Value::Null => string_elements.push("kosong".to_string()),
                _ => {
                    return Err(
                        "Teu tiasa ngagabungkeun daptar anu eusina daptar/objek/pancen."
                            .to_string(),
                    );
                }
            }
        }

        Ok(Value::String(Rc::new(string_elements.join(delimiter))))
    } else {
        Err("Aya lepat sistem: panarima sanes daptar.".to_string())
    }
}

fn array_length(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("Metode 'panjang' teu peryogi argumen.".to_string());
    }

    if let Value::Array(array) = receiver {
        Ok(Value::Integer(array.borrow().len() as i64))
    } else {
        Err("Aya lepat sistem: panarima sanes daptar.".to_string())
    }
}

fn array_map(
    vm: &mut VM,
    chunk: &Chunk,
    receiver: &Value,
    args: &[Value],
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("Metode 'petakeun' peryogi 1 argumen pancen.".to_string());
    }

    let (function_address, arity) = match &args[0] {
        Value::Function(function) => (function.address, function.arity),
        _ => return Err("Argumen 'petakeun' kedah mangrupa pancen.".to_string()),
    };

    if arity != 1 {
        return Err(format!(
            "Pancen keur 'petakeun' peryogi 1 parameter, tapi dipasihan {}.",
            arity
        ));
    }

    if let Value::Array(array) = receiver {
        let elements = array.borrow().clone();
        let mut mapped_elements = Vec::new();

        for element in elements {
            vm.stack.push(args[0].clone());
            vm.stack.push(element.clone());

            let result = vm.execute_function_sync(chunk, function_address, 1)?;
            mapped_elements.push(result);
        }

        Ok(Value::Array(Rc::new(RefCell::new(mapped_elements))))
    } else {
        Err("Aya lepat sistem: panarima sanes daptar.".to_string())
    }
}

fn array_pop(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("Metode 'candak_pengker' teu peryogi argumen.".to_string());
    }

    if let Value::Array(array) = receiver {
        match array.borrow_mut().pop() {
            Some(value) => Ok(value),
            None => Ok(Value::Null),
        }
    } else {
        Err("Aya lepat sistem: panarima sanes daptar.".to_string())
    }
}

fn array_push(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("Metode 'tambih_pengker' peryogi 1 argumen.".to_string());
    }

    if let Value::Array(array) = receiver {
        array.borrow_mut().push(args[0].clone());
        Ok(Value::Null)
    } else {
        Err("Aya lepat sistem: panarima sanes daptar.".to_string())
    }
}

fn array_reverse(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("Metode 'balikkeun' teu peryogi argumen.".to_string());
    }

    if let Value::Array(array) = receiver {
        array.borrow_mut().reverse();
        Ok(Value::Null)
    } else {
        Err("Aya lepat sistem: panarima sanes daptar.".to_string())
    }
}

fn array_shift(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("Metode 'candak_payun' teu peryogi argumen.".to_string());
    }

    if let Value::Array(array) = receiver {
        let mut elements = array.borrow_mut();

        if elements.is_empty() {
            Ok(Value::Null)
        } else {
            Ok(elements.remove(0))
        }
    } else {
        Err("Aya lepat sistem: panarima sanes daptar.".to_string())
    }
}

fn array_slice(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("Metode 'potong' peryogi 2 argumen (awal, akhir).".to_string());
    }

    if let Value::Array(array) = receiver {
        let start = match args[0] {
            Value::Integer(i) => {
                if i < 0 {
                    return Err("Indeks awal teu kenging negatif.".to_string());
                }
                i as usize
            }
            _ => return Err("Argumen 'awal' kedah mangrupa angka buleud.".to_string()),
        };

        let end = match args[1] {
            Value::Integer(i) => {
                if i < 0 {
                    return Err("Indeks akhir teu kenging negatif.".to_string());
                }
                i as usize
            }
            _ => return Err("Argumen 'akhir' kedah mangrupa angka buleud.".to_string()),
        };

        let elements = array.borrow();
        let len = elements.len();

        if start > len || end > len || start > end {
            return Err("Indeks 'potong' teu sah atanapi di luar wates daptar.".to_string());
        }

        let sliced = elements[start..end].to_vec();
        Ok(Value::Array(Rc::new(RefCell::new(sliced))))
    } else {
        Err("Aya lepat sistem: panarima sanes daptar.".to_string())
    }
}

fn array_sort(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("Metode 'susun' teu peryogi argumen.".to_string());
    }

    if let Value::Array(array) = receiver {
        let mut elements = array.borrow_mut();

        elements.sort_by(|a, b| match (a, b) {
            (Value::Integer(l), Value::Integer(r)) => l.cmp(r),
            (Value::Float(l), Value::Float(r)) => {
                l.partial_cmp(r).unwrap_or(std::cmp::Ordering::Equal)
            }
            (Value::Integer(l), Value::Float(r)) => (*l as f64)
                .partial_cmp(r)
                .unwrap_or(std::cmp::Ordering::Equal),
            (Value::Float(l), Value::Integer(r)) => l
                .partial_cmp(&(*r as f64))
                .unwrap_or(std::cmp::Ordering::Equal),
            (Value::String(l), Value::String(r)) => l.cmp(r),
            _ => std::cmp::Ordering::Equal,
        });

        Ok(Value::Null)
    } else {
        Err("Aya lepat sistem: panarima sanes daptar.".to_string())
    }
}

fn array_unshift(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("Metode 'tambih_payun' peryogi 1 argumen.".to_string());
    }

    if let Value::Array(array) = receiver {
        array.borrow_mut().insert(0, args[0].clone());
        Ok(Value::Null)
    } else {
        Err("Aya lepat sistem: panarima sanes daptar.".to_string())
    }
}

fn float_abs(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("Metode 'mutlak' teu peryogi argumen.".to_string());
    }

    if let Value::Float(float) = receiver {
        Ok(Value::Float(float.abs()))
    } else {
        Err("Aya lepat sistem: panarima sanes angka pecahan.".to_string())
    }
}

fn float_ceil(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("Metode 'ka_luhur' teu peryogi argumen.".to_string());
    }

    if let Value::Float(float) = receiver {
        Ok(Value::Integer(float.ceil() as i64))
    } else {
        Err("Aya lepat sistem: panarima sanes angka pecahan.".to_string())
    }
}

fn float_clamp(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("Metode 'watesan' peryogi 2 argumen (min, max).".to_string());
    }

    if let Value::Float(float) = receiver {
        let min = match args[0] {
            Value::Float(m) => m,
            Value::Integer(m) => m as f64,
            _ => {
                return Err(
                    "Argumen 'min' kedah mangrupa angka pecahan atanapi buleud.".to_string()
                );
            }
        };

        let max = match args[1] {
            Value::Float(m) => m,
            Value::Integer(m) => m as f64,
            _ => {
                return Err(
                    "Argumen 'max' kedah mangrupa angka pecahan atanapi buleud.".to_string()
                );
            }
        };

        if min > max {
            return Err("Argumen 'min' teu kenging langkung ti 'max'.".to_string());
        }

        Ok(Value::Float(float.clamp(min, max)))
    } else {
        Err("Aya lepat sistem: panarima sanes angka pecahan.".to_string())
    }
}

fn float_floor(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("Metode 'ka_handap' teu peryogi argumen.".to_string());
    }

    if let Value::Float(float) = receiver {
        Ok(Value::Integer(float.floor() as i64))
    } else {
        Err("Aya lepat sistem: panarima sanes angka pecahan.".to_string())
    }
}

fn float_round(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("Metode 'buleudkeun' teu peryogi argumen.".to_string());
    }

    if let Value::Float(float) = receiver {
        Ok(Value::Integer(float.round() as i64))
    } else {
        Err("Aya lepat sistem: panarima sanes angka pecahan.".to_string())
    }
}

fn float_to_string(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("Metode 'janten_teks' teu peryogi argumen.".to_string());
    }

    if let Value::Float(float) = receiver {
        Ok(Value::String(Rc::new(float.to_string())))
    } else {
        Err("Aya lepat sistem: panarima sanes angka pecahan.".to_string())
    }
}

fn integer_abs(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("Metode 'mutlak' teu peryogi argumen.".to_string());
    }

    if let Value::Integer(integer) = receiver {
        Ok(Value::Integer(integer.abs()))
    } else {
        Err("Aya lepat sistem: panarima sanes angka buleud.".to_string())
    }
}

fn integer_clamp(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("Metode 'watesan' peryogi 2 argumen (min, max).".to_string());
    }

    if let Value::Integer(integer) = receiver {
        let min = match args[0] {
            Value::Integer(m) => m,
            _ => return Err("Argumen 'min' kedah mangrupa angka buleud.".to_string()),
        };

        let max = match args[1] {
            Value::Integer(m) => m,
            _ => return Err("Argumen 'max' kedah mangrupa angka buleud.".to_string()),
        };

        if min > max {
            return Err("Argumen 'min' teu kenging langkung ti 'max'.".to_string());
        }

        Ok(Value::Integer((*integer).clamp(min, max)))
    } else {
        Err("Aya lepat sistem: panarima sanes angka buleud.".to_string())
    }
}

fn integer_to_float(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("Metode 'janten_desimal' teu peryogi argumen.".to_string());
    }

    if let Value::Integer(integer) = receiver {
        Ok(Value::Float(*integer as f64))
    } else {
        Err("Aya lepat sistem: panarima sanes angka buleud.".to_string())
    }
}

fn integer_to_string(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("Metode 'janten_teks' teu peryogi argumen.".to_string());
    }

    if let Value::Integer(integer) = receiver {
        Ok(Value::String(Rc::new(integer.to_string())))
    } else {
        Err("Aya lepat sistem: panarima sanes angka buleud.".to_string())
    }
}

fn object_has(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("Metode 'ngagaduhan' peryogi 1 argumen teks.".to_string());
    }

    if let Value::Object(object) = receiver {
        let key = match &args[0] {
            Value::String(s) => s.as_str(),
            _ => return Err("Argumen 'ngagaduhan' kedah mangrupa teks.".to_string()),
        };

        let map = object.borrow();
        Ok(Value::Boolean(map.contains_key(key)))
    } else {
        Err("Aya lepat sistem: panarima sanes objek.".to_string())
    }
}

fn object_keys(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("Metode 'konci' teu peryogi argumen.".to_string());
    }

    if let Value::Object(object) = receiver {
        let map = object.borrow();
        let keys: Vec<Value> = map
            .keys()
            .map(|k| Value::String(Rc::new(k.clone())))
            .collect();

        Ok(Value::Array(Rc::new(RefCell::new(keys))))
    } else {
        Err("Aya lepat sistem: panarima sanes objek.".to_string())
    }
}

fn object_remove(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("Metode 'piceun' peryogi 1 argumen teks.".to_string());
    }

    if let Value::Object(object) = receiver {
        let key = match &args[0] {
            Value::String(s) => s.as_str(),
            _ => return Err("Argumen 'piceun' kedah mangrupa teks.".to_string()),
        };

        let mut map = object.borrow_mut();

        match map.remove(key) {
            Some(removed_value) => Ok(removed_value),
            None => Ok(Value::Null),
        }
    } else {
        Err("Aya lepat sistem: panarima sanes objek.".to_string())
    }
}

fn object_size(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("Metode 'jumlah' teu peryogi argumen.".to_string());
    }

    if let Value::Object(object) = receiver {
        let map = object.borrow();
        Ok(Value::Integer(map.len() as i64))
    } else {
        Err("Aya lepat sistem: panarima sanes objek.".to_string())
    }
}

fn object_values(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("Metode 'eusi' teu peryogi argumen.".to_string());
    }

    if let Value::Object(object) = receiver {
        let map = object.borrow();
        let values: Vec<Value> = map.values().cloned().collect();

        Ok(Value::Array(Rc::new(RefCell::new(values))))
    } else {
        Err("Aya lepat sistem: panarima sanes objek.".to_string())
    }
}

fn string_char_at(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("Metode 'hurup_ka' peryogi 1 argumen angka buleud.".to_string());
    }

    if let Value::String(string) = receiver {
        let index = match args[0] {
            Value::Integer(i) => {
                if i < 0 {
                    return Err("Indeks 'hurup_ka' teu kenging negatif.".to_string());
                }
                i as usize
            }
            _ => return Err("Argumen indeks kedah mangrupa angka buleud.".to_string()),
        };

        match string.chars().nth(index) {
            Some(c) => Ok(Value::String(Rc::new(c.to_string()))),
            None => Err("Indeks 'hurup_ka' di luar wates teks.".to_string()),
        }
    } else {
        Err("Aya lepat sistem: panarima sanes teks.".to_string())
    }
}

fn string_contains(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("Metode 'ngandung' peryogi 1 argumen.".to_string());
    }

    if let Value::String(string) = receiver {
        let search_string = match &args[0] {
            Value::String(s) => s.as_str(),
            _ => return Err("Argumen pamilarian kedah mangrupa teks.".to_string()),
        };

        Ok(Value::Boolean(string.contains(search_string)))
    } else {
        Err("Aya lepat sistem: panarima sanes teks.".to_string())
    }
}

fn string_index_of(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("Metode 'posisi_ka' peryogi 1 argumen teks.".to_string());
    }

    if let Value::String(string) = receiver {
        let search_string = match &args[0] {
            Value::String(s) => s.as_str(),
            _ => return Err("Argumen pamilarian kedah mangrupa teks.".to_string()),
        };

        match string.find(search_string) {
            Some(byte_index) => {
                let char_index = string[..byte_index].chars().count();
                Ok(Value::Integer(char_index as i64))
            }
            None => Ok(Value::Integer(-1)),
        }
    } else {
        Err("Aya lepat sistem: panarima sanes teks.".to_string())
    }
}

fn string_length(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("Metode 'panjang' teu peryogi argumen.".to_string());
    }

    if let Value::String(string) = receiver {
        Ok(Value::Integer(string.chars().count() as i64))
    } else {
        Err("Aya lepat sistem: panarima sanes teks.".to_string())
    }
}

fn string_lowercase(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("Metode 'aksara_alit' teu peryogi argumen.".to_string());
    }

    if let Value::String(string) = receiver {
        Ok(Value::String(Rc::new(string.to_lowercase())))
    } else {
        Err("Aya lepat sistem: panarima sanes teks.".to_string())
    }
}

fn string_replace(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("Metode 'gentos' peryogi 2 argumen (lami, anyar).".to_string());
    }

    if let Value::String(string) = receiver {
        let old_string = match &args[0] {
            Value::String(s) => s.as_str(),
            _ => return Err("Argumen lami kedah mangrupa teks.".to_string()),
        };

        let new_string = match &args[1] {
            Value::String(s) => s.as_str(),
            _ => return Err("Argumen anyar kedah mangrupa teks.".to_string()),
        };

        Ok(Value::String(Rc::new(
            string.replace(old_string, new_string),
        )))
    } else {
        Err("Aya lepat sistem: panarima sanes teks.".to_string())
    }
}

fn string_reverse(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("Metode 'balikkeun' teu peryogi argumen.".to_string());
    }

    if let Value::String(string) = receiver {
        let reversed: String = string.chars().rev().collect();
        Ok(Value::String(Rc::new(reversed)))
    } else {
        Err("Aya lepat sistem: panarima sanes teks.".to_string())
    }
}

fn string_slice(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("Metode 'potong' peryogi 2 argumen (awal, akhir).".to_string());
    }

    if let Value::String(string) = receiver {
        let start = match args[0] {
            Value::Integer(i) => i as usize,
            _ => return Err("Argumen 'awal' kedah mangrupa angka buleud.".to_string()),
        };

        let end = match args[1] {
            Value::Integer(i) => i as usize,
            _ => return Err("Argumen 'akhir' kedah mangrupa angka buleud.".to_string()),
        };

        let len = string.chars().count();
        if start > len || end > len || start > end {
            return Err("Indeks 'potong' teu sah atanapi di luar wates teks.".to_string());
        }

        let sliced_string: String = string.chars().skip(start).take(end - start).collect();
        Ok(Value::String(Rc::new(sliced_string)))
    } else {
        Err("Aya lepat sistem: panarima sanes teks.".to_string())
    }
}

fn string_split(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("Metode 'pisahkeun' peryogi 1 argumen pamisa.".to_string());
    }

    if let Value::String(string) = receiver {
        let delimiter = match &args[0] {
            Value::String(s) => s.as_str(),
            _ => return Err("Argumen pamisa kedah mangrupa teks.".to_string()),
        };

        let parts: Vec<Value> = string
            .split(delimiter)
            .map(|part| Value::String(Rc::new(part.to_string())))
            .collect();

        Ok(Value::Array(Rc::new(RefCell::new(parts))))
    } else {
        Err("Aya lepat sistem: panarima sanes teks.".to_string())
    }
}

fn string_to_float(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("Metode 'janten_desimal' teu peryogi argumen.".to_string());
    }

    if let Value::String(string) = receiver {
        match string.trim().parse::<f64>() {
            Ok(f) => Ok(Value::Float(f)),
            Err(_) => Err(format!(
                "Gagal ngarobih teks '{}' janten angka pecahan.",
                string
            )),
        }
    } else {
        Err("Aya lepat sistem: panarima sanes teks.".to_string())
    }
}

fn string_to_integer(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("Metode 'janten_angka' teu peryogi argumen.".to_string());
    }

    if let Value::String(string) = receiver {
        match string.trim().parse::<i64>() {
            Ok(i) => Ok(Value::Integer(i)),
            Err(_) => Err(format!(
                "Gagal ngarobih teks '{}' janten angka buleud.",
                string
            )),
        }
    } else {
        Err("Aya lepat sistem: panarima sanes teks.".to_string())
    }
}

fn string_trim(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("Metode 'rapikeun' teu peryogi argumen.".to_string());
    }

    if let Value::String(string) = receiver {
        Ok(Value::String(Rc::new(string.trim().to_string())))
    } else {
        Err("Aya lepat sistem: panarima sanes teks.".to_string())
    }
}

fn string_uppercase(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("Metode 'aksara_ageung' teu peryogi argumen.".to_string());
    }

    if let Value::String(string) = receiver {
        Ok(Value::String(Rc::new(string.to_uppercase())))
    } else {
        Err("Aya lepat sistem: panarima sanes teks.".to_string())
    }
}
