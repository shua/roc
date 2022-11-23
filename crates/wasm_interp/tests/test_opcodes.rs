#![cfg(test)]

use bumpalo::{collections::Vec, Bump};
use roc_wasm_interp::{Action, ExecutionState, ValueStack};
use roc_wasm_module::{
    opcodes::OpCode, SerialBuffer, Serialize, Signature, Value, ValueType, WasmModule,
};

fn default_state<'a>(arena: &'a Bump) -> ExecutionState {
    let pages = 1;
    let program_counter = 0;
    let globals = [];
    ExecutionState::new(&arena, pages, program_counter, globals)
}

// #[test]
// fn test_block() {}

// #[test]
// fn test_loop() {}

// #[test]
// fn test_if() {}

// #[test]
// fn test_else() {}

// #[test]
// fn test_end() {}

// #[test]
// fn test_br() {}

// #[test]
// fn test_brif() {}

// #[test]
// fn test_brtable() {}

#[test]
fn test_call_return_no_args() {
    let arena = Bump::new();
    let mut state = default_state(&arena);
    let mut module = WasmModule::new(&arena);
    let sig = || Signature {
        param_types: Vec::new_in(&arena),
        ret_type: Some(ValueType::I32),
    };
    module.add_function_signature(sig());
    module.add_function_signature(sig());

    let buffer = &mut module.code.bytes;

    // Function 0
    let func0_offset = buffer.len() as u32;
    module.code.function_offsets.push(func0_offset);
    buffer.encode_u32(4); // function byte length
    buffer.encode_u32(0); // no locals
    let func0_first_instruction = buffer.len();
    OpCode::CALL.serialize(buffer);
    buffer.encode_u32(1); // call function 1
    OpCode::END.serialize(buffer);

    // Function 1
    let func1_offset = buffer.len() as u32;
    module.code.function_offsets.push(func1_offset);
    buffer.encode_u32(6); // function byte length
    buffer.encode_u32(0); // no locals
    OpCode::I32CONST.serialize(buffer);
    buffer.encode_i32(12345);
    OpCode::END.serialize(buffer);

    state.program_counter = func0_first_instruction;

    state.execute_next_instruction(&module); // [func0] call 1
    state.execute_next_instruction(&module); // [func1] i32.const
    state.execute_next_instruction(&module); // [func1] end
    state.execute_next_instruction(&module); // [func0] end

    assert_eq!(state.value_stack.peek(), Value::I32(12345));
}

#[test]
fn test_call_return_with_args() {
    let arena = Bump::new();
    let mut state = default_state(&arena);
    let mut module = WasmModule::new(&arena);

    // Function 0: calculate 2+2
    let func0_offset = module.code.bytes.len() as u32;
    module.code.function_offsets.push(func0_offset);
    module.add_function_signature(Signature {
        param_types: bumpalo::vec![in &arena;],
        ret_type: Some(ValueType::I32),
    });
    [
        0, // no locals
        OpCode::I32CONST as u8,
        2,
        OpCode::I32CONST as u8,
        2,
        OpCode::CALL as u8,
        1,
        OpCode::END as u8,
    ]
    .serialize(&mut module.code.bytes);
    let func0_first_instruction = func0_offset + 2;

    // Function 1: add two numbers
    let func1_offset = module.code.bytes.len() as u32;
    module.code.function_offsets.push(func1_offset);
    module.add_function_signature(Signature {
        param_types: bumpalo::vec![in &arena; ValueType::I32, ValueType::I32],
        ret_type: Some(ValueType::I32),
    });
    [
        0, // no locals
        OpCode::GETLOCAL as u8,
        0,
        OpCode::GETLOCAL as u8,
        1,
        OpCode::I32ADD as u8,
        OpCode::END as u8,
    ]
    .serialize(&mut module.code.bytes);

    state.program_counter = func0_first_instruction as usize;

    while let Action::Continue = state.execute_next_instruction(&module) {}

    assert_eq!(state.value_stack.peek(), Value::I32(4));
}

// #[test]
// fn test_callindirect() {}

// #[test]
// fn test_drop() {}

// #[test]
// fn test_select() {}

#[test]
fn test_set_get_local() {
    let arena = Bump::new();
    let mut state = default_state(&arena);
    let mut module = WasmModule::new(&arena);
    let mut vs = ValueStack::new(&arena);

    let mut buffer = vec![];
    let mut cursor = 0;
    [
        (1u32, ValueType::F32),
        (1u32, ValueType::F64),
        (1u32, ValueType::I32),
        (1u32, ValueType::I64),
    ]
    .serialize(&mut buffer);
    state
        .call_stack
        .push_frame(0x1234, 0, 0, &mut vs, &buffer, &mut cursor);

    module.code.bytes.push(OpCode::I32CONST as u8);
    module.code.bytes.encode_i32(12345);
    module.code.bytes.push(OpCode::SETLOCAL as u8);
    module.code.bytes.encode_u32(2);

    module.code.bytes.push(OpCode::GETLOCAL as u8);
    module.code.bytes.encode_u32(2);

    state.execute_next_instruction(&module);
    state.execute_next_instruction(&module);
    state.execute_next_instruction(&module);
    assert_eq!(state.value_stack.len(), 1);
    assert_eq!(state.value_stack.pop(), Value::I32(12345));
}

#[test]
fn test_tee_get_local() {
    let arena = Bump::new();
    let mut state = default_state(&arena);
    let mut module = WasmModule::new(&arena);
    let mut vs = ValueStack::new(&arena);

    let mut buffer = vec![];
    let mut cursor = 0;
    [
        (1u32, ValueType::F32),
        (1u32, ValueType::F64),
        (1u32, ValueType::I32),
        (1u32, ValueType::I64),
    ]
    .serialize(&mut buffer);
    state
        .call_stack
        .push_frame(0x1234, 0, 0, &mut vs, &buffer, &mut cursor);

    module.code.bytes.push(OpCode::I32CONST as u8);
    module.code.bytes.encode_i32(12345);
    module.code.bytes.push(OpCode::TEELOCAL as u8);
    module.code.bytes.encode_u32(2);

    module.code.bytes.push(OpCode::GETLOCAL as u8);
    module.code.bytes.encode_u32(2);

    state.execute_next_instruction(&module);
    state.execute_next_instruction(&module);
    state.execute_next_instruction(&module);
    assert_eq!(state.value_stack.len(), 2);
    assert_eq!(state.value_stack.pop(), Value::I32(12345));
    assert_eq!(state.value_stack.pop(), Value::I32(12345));
}

#[test]
fn test_global() {
    let arena = Bump::new();
    let mut state = default_state(&arena);
    state
        .globals
        .extend_from_slice(&[Value::F64(1.11), Value::I32(222), Value::F64(3.33)]);
    let mut module = WasmModule::new(&arena);

    module.code.bytes.push(OpCode::GETGLOBAL as u8);
    module.code.bytes.encode_u32(1);
    module.code.bytes.push(OpCode::I32CONST as u8);
    module.code.bytes.encode_i32(555);
    module.code.bytes.push(OpCode::SETGLOBAL as u8);
    module.code.bytes.encode_u32(1);
    module.code.bytes.push(OpCode::GETGLOBAL as u8);
    module.code.bytes.encode_u32(1);

    state.execute_next_instruction(&module);
    state.execute_next_instruction(&module);
    state.execute_next_instruction(&module);
    state.execute_next_instruction(&module);
    assert_eq!(state.value_stack.len(), 2);
    assert_eq!(state.value_stack.pop(), Value::I32(555));
    assert_eq!(state.value_stack.pop(), Value::I32(222));
}

// #[test]
// fn test_i32load() {}

// #[test]
// fn test_i64load() {}

// #[test]
// fn test_f32load() {}

// #[test]
// fn test_f64load() {}

// #[test]
// fn test_i32load8s() {}

// #[test]
// fn test_i32load8u() {}

// #[test]
// fn test_i32load16s() {}

// #[test]
// fn test_i32load16u() {}

// #[test]
// fn test_i64load8s() {}

// #[test]
// fn test_i64load8u() {}

// #[test]
// fn test_i64load16s() {}

// #[test]
// fn test_i64load16u() {}

// #[test]
// fn test_i64load32s() {}

// #[test]
// fn test_i64load32u() {}

// #[test]
// fn test_i32store() {}

// #[test]
// fn test_i64store() {}

// #[test]
// fn test_f32store() {}

// #[test]
// fn test_f64store() {}

// #[test]
// fn test_i32store8() {}

// #[test]
// fn test_i32store16() {}

// #[test]
// fn test_i64store8() {}

// #[test]
// fn test_i64store16() {}

// #[test]
// fn test_i64store32() {}

// #[test]
// fn test_currentmemory() {}

// #[test]
// fn test_growmemory() {}

#[test]
fn test_i32const() {
    let arena = Bump::new();
    let mut state = default_state(&arena);
    let mut module = WasmModule::new(&arena);

    module.code.bytes.push(OpCode::I32CONST as u8);
    module.code.bytes.encode_i32(12345);

    state.execute_next_instruction(&module);
    assert_eq!(state.value_stack.pop(), Value::I32(12345))
}

#[test]
fn test_i64const() {
    let arena = Bump::new();
    let mut state = default_state(&arena);
    let mut module = WasmModule::new(&arena);

    module.code.bytes.push(OpCode::I64CONST as u8);
    module.code.bytes.encode_i64(1234567890);

    state.execute_next_instruction(&module);
    assert_eq!(state.value_stack.pop(), Value::I64(1234567890))
}

#[test]
fn test_f32const() {
    let arena = Bump::new();
    let mut state = default_state(&arena);
    let mut module = WasmModule::new(&arena);

    module.code.bytes.push(OpCode::F32CONST as u8);
    module.code.bytes.encode_f32(123.45);

    state.execute_next_instruction(&module);
    assert_eq!(state.value_stack.pop(), Value::F32(123.45))
}

#[test]
fn test_f64const() {
    let arena = Bump::new();
    let mut state = default_state(&arena);
    let mut module = WasmModule::new(&arena);

    module.code.bytes.push(OpCode::F64CONST as u8);
    module.code.bytes.encode_f64(12345.67890);

    state.execute_next_instruction(&module);
    assert_eq!(state.value_stack.pop(), Value::F64(12345.67890))
}

// #[test]
// fn test_i32eqz() {}

// #[test]
// fn test_i32eq() {}

// #[test]
// fn test_i32ne() {}

// #[test]
// fn test_i32lts() {}

// #[test]
// fn test_i32ltu() {}

// #[test]
// fn test_i32gts() {}

// #[test]
// fn test_i32gtu() {}

// #[test]
// fn test_i32les() {}

// #[test]
// fn test_i32leu() {}

// #[test]
// fn test_i32ges() {}

// #[test]
// fn test_i32geu() {}

// #[test]
// fn test_i64eqz() {}

// #[test]
// fn test_i64eq() {}

// #[test]
// fn test_i64ne() {}

// #[test]
// fn test_i64lts() {}

// #[test]
// fn test_i64ltu() {}

// #[test]
// fn test_i64gts() {}

// #[test]
// fn test_i64gtu() {}

// #[test]
// fn test_i64les() {}

// #[test]
// fn test_i64leu() {}

// #[test]
// fn test_i64ges() {}

// #[test]
// fn test_i64geu() {}

// #[test]
// fn test_f32eq() {}

// #[test]
// fn test_f32ne() {}

// #[test]
// fn test_f32lt() {}

// #[test]
// fn test_f32gt() {}

// #[test]
// fn test_f32le() {}

// #[test]
// fn test_f32ge() {}

// #[test]
// fn test_f64eq() {}

// #[test]
// fn test_f64ne() {}

// #[test]
// fn test_f64lt() {}

// #[test]
// fn test_f64gt() {}

// #[test]
// fn test_f64le() {}

// #[test]
// fn test_f64ge() {}

// #[test]
// fn test_i32clz() {}

// #[test]
// fn test_i32ctz() {}

// #[test]
// fn test_i32popcnt() {}

#[test]
fn test_i32add() {
    let arena = Bump::new();
    let mut state = default_state(&arena);
    let mut module = WasmModule::new(&arena);

    module.code.bytes.push(OpCode::I32CONST as u8);
    module.code.bytes.encode_i32(123);
    module.code.bytes.push(OpCode::I32CONST as u8);
    module.code.bytes.encode_i32(321);
    module.code.bytes.push(OpCode::I32ADD as u8);

    state.execute_next_instruction(&module);
    state.execute_next_instruction(&module);
    state.execute_next_instruction(&module);
    assert_eq!(state.value_stack.pop(), Value::I32(444))
}

#[test]
fn test_i32sub() {
    let arena = Bump::new();
    let mut state = default_state(&arena);
    let mut module = WasmModule::new(&arena);

    module.code.bytes.push(OpCode::I32CONST as u8);
    module.code.bytes.encode_i32(123);
    module.code.bytes.push(OpCode::I32CONST as u8);
    module.code.bytes.encode_i32(321);
    module.code.bytes.push(OpCode::I32SUB as u8);

    state.execute_next_instruction(&module);
    state.execute_next_instruction(&module);
    state.execute_next_instruction(&module);
    assert_eq!(state.value_stack.pop(), Value::I32(-198))
}

#[test]
fn test_i32mul() {
    let arena = Bump::new();
    let mut state = default_state(&arena);
    let mut module = WasmModule::new(&arena);

    module.code.bytes.push(OpCode::I32CONST as u8);
    module.code.bytes.encode_i32(123);
    module.code.bytes.push(OpCode::I32CONST as u8);
    module.code.bytes.encode_i32(321);
    module.code.bytes.push(OpCode::I32MUL as u8);

    state.execute_next_instruction(&module);
    state.execute_next_instruction(&module);
    state.execute_next_instruction(&module);
    assert_eq!(state.value_stack.pop(), Value::I32(39483))
}

// #[test]
// fn test_i32divs() {}

// #[test]
// fn test_i32divu() {}

// #[test]
// fn test_i32rems() {}

// #[test]
// fn test_i32remu() {}

// #[test]
// fn test_i32and() {}

// #[test]
// fn test_i32or() {}

// #[test]
// fn test_i32xor() {}

// #[test]
// fn test_i32shl() {}

// #[test]
// fn test_i32shrs() {}

// #[test]
// fn test_i32shru() {}

// #[test]
// fn test_i32rotl() {}

// #[test]
// fn test_i32rotr() {}

// #[test]
// fn test_i64clz() {}

// #[test]
// fn test_i64ctz() {}

// #[test]
// fn test_i64popcnt() {}

// #[test]
// fn test_i64add() {}

// #[test]
// fn test_i64sub() {}

// #[test]
// fn test_i64mul() {}

// #[test]
// fn test_i64divs() {}

// #[test]
// fn test_i64divu() {}

// #[test]
// fn test_i64rems() {}

// #[test]
// fn test_i64remu() {}

// #[test]
// fn test_i64and() {}

// #[test]
// fn test_i64or() {}

// #[test]
// fn test_i64xor() {}

// #[test]
// fn test_i64shl() {}

// #[test]
// fn test_i64shrs() {}

// #[test]
// fn test_i64shru() {}

// #[test]
// fn test_i64rotl() {}

// #[test]
// fn test_i64rotr() {}

// #[test]
// fn test_f32abs() {}

// #[test]
// fn test_f32neg() {}

// #[test]
// fn test_f32ceil() {}

// #[test]
// fn test_f32floor() {}

// #[test]
// fn test_f32trunc() {}

// #[test]
// fn test_f32nearest() {}

// #[test]
// fn test_f32sqrt() {}

// #[test]
// fn test_f32add() {}

// #[test]
// fn test_f32sub() {}

// #[test]
// fn test_f32mul() {}

// #[test]
// fn test_f32div() {}

// #[test]
// fn test_f32min() {}

// #[test]
// fn test_f32max() {}

// #[test]
// fn test_f32copysign() {}

// #[test]
// fn test_f64abs() {}

// #[test]
// fn test_f64neg() {}

// #[test]
// fn test_f64ceil() {}

// #[test]
// fn test_f64floor() {}

// #[test]
// fn test_f64trunc() {}

// #[test]
// fn test_f64nearest() {}

// #[test]
// fn test_f64sqrt() {}

// #[test]
// fn test_f64add() {}

// #[test]
// fn test_f64sub() {}

// #[test]
// fn test_f64mul() {}

// #[test]
// fn test_f64div() {}

// #[test]
// fn test_f64min() {}

// #[test]
// fn test_f64max() {}

// #[test]
// fn test_f64copysign() {}

// #[test]
// fn test_i32wrapi64() {}

// #[test]
// fn test_i32truncsf32() {}

// #[test]
// fn test_i32truncuf32() {}

// #[test]
// fn test_i32truncsf64() {}

// #[test]
// fn test_i32truncuf64() {}

// #[test]
// fn test_i64extendsi32() {}

// #[test]
// fn test_i64extendui32() {}

// #[test]
// fn test_i64truncsf32() {}

// #[test]
// fn test_i64truncuf32() {}

// #[test]
// fn test_i64truncsf64() {}

// #[test]
// fn test_i64truncuf64() {}

// #[test]
// fn test_f32convertsi32() {}

// #[test]
// fn test_f32convertui32() {}

// #[test]
// fn test_f32convertsi64() {}

// #[test]
// fn test_f32convertui64() {}

// #[test]
// fn test_f32demotef64() {}

// #[test]
// fn test_f64convertsi32() {}

// #[test]
// fn test_f64convertui32() {}

// #[test]
// fn test_f64convertsi64() {}

// #[test]
// fn test_f64convertui64() {}

// #[test]
// fn test_f64promotef32() {}

// #[test]
// fn test_i32reinterpretf32() {}

// #[test]
// fn test_i64reinterpretf64() {}

// #[test]
// fn test_f32reinterpreti32() {}

// #[test]
// fn test_f64reinterpreti64() {}
