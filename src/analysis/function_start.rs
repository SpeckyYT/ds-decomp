use unarm::{
    args::{Argument, Reg, Register},
    arm, thumb, Ins, ParsedIns,
};

pub fn is_valid_function_start_arm(_address: u32, ins: arm::Ins, parsed_ins: &ParsedIns) -> bool {
    if ins.op == arm::Opcode::Illegal || parsed_ins.is_illegal() {
        false
    } else if ins.has_cond() && ins.modifier_cond() != arm::Cond::Al {
        false
    } else {
        true
    }
}

pub fn is_valid_function_start_thumb(_address: u32, ins: thumb::Ins, parsed_ins: &ParsedIns) -> bool {
    if matches!(ins.op, thumb::Opcode::Illegal | thumb::Opcode::Bl | thumb::Opcode::BlH) || parsed_ins.is_illegal() {
        return false;
    }

    let args = &parsed_ins.args;
    match (parsed_ins.mnemonic, args[0], args[1], args[2], args[3]) {
        ("mov", Argument::Reg(Reg { reg: dst, .. }), Argument::Reg(Reg { reg: src, .. }), Argument::None, Argument::None)
        | ("movs", Argument::Reg(Reg { reg: dst, .. }), Argument::Reg(Reg { reg: src, .. }), Argument::None, Argument::None)
            if src == dst =>
        {
            // Useless mov
            false
        }
        (
            "lsl",
            Argument::Reg(Reg { reg: dst, .. }),
            Argument::Reg(Reg { reg: src, .. }),
            Argument::UImm(0),
            Argument::None,
        )
        | (
            "lsls",
            Argument::Reg(Reg { reg: dst, .. }),
            Argument::Reg(Reg { reg: src, .. }),
            Argument::UImm(0),
            Argument::None,
        ) if src == dst => {
            // Useless data op
            false
        }
        ("lsl", Argument::Reg(_), Argument::Reg(Reg { reg: src, .. }), Argument::UImm(shift), Argument::None)
        | ("lsls", Argument::Reg(_), Argument::Reg(Reg { reg: src, .. }), Argument::UImm(shift), Argument::None)
        | ("lsr", Argument::Reg(_), Argument::Reg(Reg { reg: src, .. }), Argument::UImm(shift), Argument::None)
        | ("lsrs", Argument::Reg(_), Argument::Reg(Reg { reg: src, .. }), Argument::UImm(shift), Argument::None)
            if src == Register::R0 && (shift % 4) == 0 =>
        {
            // Table of bytes with values 0-7 got interpreted as Thumb code
            false
        }
        ("ldr", Argument::Reg(_), Argument::Reg(Reg { deref: true, reg, .. }), _, _)
        | ("ldrh", Argument::Reg(_), Argument::Reg(Reg { deref: true, reg, .. }), _, _)
        | ("ldrb", Argument::Reg(_), Argument::Reg(Reg { deref: true, reg, .. }), _, _)
            if !matches!(reg, Register::R0 | Register::R1 | Register::R2 | Register::R3 | Register::Sp | Register::Pc) =>
        {
            // Load base must be an argument register, SP or PC
            false
        }
        ("strh", Argument::Reg(Reg { reg, .. }), Argument::Reg(Reg { deref: true, reg: base, .. }), _, _)
        | ("strb", Argument::Reg(Reg { reg, .. }), Argument::Reg(Reg { deref: true, reg: base, .. }), _, _)
            if base == reg =>
        {
            // Weird self reference:
            // *ptr = (u16) ptr;
            // *ptr = (u8) ptr;
            false
        }
        _ => true,
    }
}

pub fn is_valid_function_start(address: u32, ins: Ins, parsed_ins: &ParsedIns) -> bool {
    match ins {
        Ins::Arm(ins) => is_valid_function_start_arm(address, ins, parsed_ins),
        Ins::Thumb(ins) => is_valid_function_start_thumb(address, ins, parsed_ins),
        Ins::Data => false,
    }
}
