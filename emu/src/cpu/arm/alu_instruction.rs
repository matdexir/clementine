use serde::{Deserialize, Serialize};

use crate::bitwise::Bits;
use crate::cpu::flags::ShiftKind;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum ArmModeAluInstr {
    And = 0x0,
    Eor = 0x1,
    Sub = 0x2,
    Rsb = 0x3,
    Add = 0x4,
    Adc = 0x5,
    Sbc = 0x6,
    Rsc = 0x7,
    Tst = 0x8,
    Teq = 0x9,
    Cmp = 0xA,
    Cmn = 0xB,
    Orr = 0xC,
    Mov = 0xD,
    Bic = 0xE,
    Mvn = 0xF,
}

impl std::fmt::Display for ArmModeAluInstr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::And => f.write_str("AND"),
            Self::Eor => f.write_str("EOR"),
            Self::Sub => f.write_str("SUB"),
            Self::Rsb => f.write_str("RSB"),
            Self::Add => f.write_str("ADD"),
            Self::Adc => f.write_str("ADC"),
            Self::Sbc => f.write_str("SBC"),
            Self::Rsc => f.write_str("RSC"),
            Self::Tst => f.write_str("TST"),
            Self::Teq => f.write_str("TEQ"),
            Self::Cmp => f.write_str("CMP"),
            Self::Cmn => f.write_str("CMN"),
            Self::Orr => f.write_str("ORR"),
            Self::Mov => f.write_str("MOV"),
            Self::Bic => f.write_str("BIC"),
            Self::Mvn => f.write_str("MVN"),
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
pub enum AIKind {
    Logical,
    Arithmetic,
}

pub trait Kind {
    fn kind(&self) -> AIKind;
}

impl Kind for ArmModeAluInstr {
    fn kind(&self) -> AIKind {
        match &self {
            Self::And
            | Self::Eor
            | Self::Tst
            | Self::Teq
            | Self::Orr
            | Self::Mov
            | Self::Bic
            | Self::Mvn => AIKind::Logical,
            Self::Sub
            | Self::Rsb
            | Self::Add
            | Self::Adc
            | Self::Sbc
            | Self::Rsc
            | Self::Cmp
            | Self::Cmn => AIKind::Arithmetic,
        }
    }
}

impl From<u32> for ArmModeAluInstr {
    fn from(alu_op_code: u32) -> Self {
        match alu_op_code {
            0x0 => Self::And,
            0x1 => Self::Eor,
            0x2 => Self::Sub,
            0x3 => Self::Rsb,
            0x4 => Self::Add,
            0x5 => Self::Adc,
            0x6 => Self::Sbc,
            0x7 => Self::Rsc,
            0x8 => Self::Tst,
            0x9 => Self::Teq,
            0xA => Self::Cmp,
            0xB => Self::Cmn,
            0xC => Self::Orr,
            0xD => Self::Mov,
            0xE => Self::Bic,
            0xF => Self::Mvn,
            _ => unreachable!(),
        }
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default)]
pub struct ArithmeticOpResult {
    pub result: u32,
    pub carry: bool,
    pub overflow: bool,
    pub sign: bool,
    pub zero: bool,
}

pub fn shift(kind: ShiftKind, shift_amount: u32, rm: u32, carry: bool) -> ArithmeticOpResult {
    match kind {
        ShiftKind::Lsl => {
            match shift_amount {
                // LSL#0: No shift performed, ie. directly value=Rm, the C flag is NOT affected.
                0 => ArithmeticOpResult {
                    result: rm,
                    carry,
                    ..Default::default()
                },
                // LSL#1..32: Normal left logical shift
                1..=32 => {
                    // In Rust, when you use the << operator to shift a value to the left, the behavior is defined modulo the number of bits in the type.
                    // For a u32, there are 32 bits, so any left shift operation with a shift amount greater than or equal to 32 will wrap around and behave as
                    // if the shift amount is reduced modulo 32.
                    // So when you do 1 << 32 with a u32 in Rust, it is equivalent to 1 << (32 % 32), which is 1 << 0.
                    // Shifting a value 0 bits to the left is equivalent to the original value, so you get 1.
                    let rm = rm as u64;
                    let result = (rm << shift_amount) as u32;
                    ArithmeticOpResult {
                        result,
                        carry: rm.get_bit((32 - shift_amount).try_into().unwrap()),
                        ..Default::default()
                    }
                }
                // LSL#33...: Result is 0 and carry is 0
                _ => ArithmeticOpResult {
                    carry: false,
                    ..Default::default()
                },
            }
        }
        ShiftKind::Lsr => {
            match shift_amount {
                // LSR#0 is used to encode LSR#32, it has 0 result and carry equal to bit 31 of Rm
                0 => ArithmeticOpResult {
                    result: 0,
                    carry: rm.get_bit(31),
                    ..Default::default()
                },
                // LSR#1..32: Normal right logical shift
                1..=32 => {
                    // We do the shift in u64 for the same reason as above.
                    let rm = rm as u64;
                    let result = (rm >> shift_amount) as u32;

                    ArithmeticOpResult {
                        result,
                        carry: rm.get_bit((shift_amount - 1).try_into().unwrap()),
                        ..Default::default()
                    }
                }
                _ => ArithmeticOpResult {
                    result: 0,
                    carry: false,
                    ..Default::default()
                },
            }
        }
        ShiftKind::Asr => match shift_amount {
            1..=31 => ArithmeticOpResult {
                result: ((rm as i32) >> shift_amount) as u32,
                carry: rm.get_bit((shift_amount - 1).try_into().unwrap()),
                ..Default::default()
            },
            _ => ArithmeticOpResult {
                result: ((rm as i32) >> 31) as u32,
                carry: rm.get_bit(31),
                ..Default::default()
            },
        },
        ShiftKind::Ror => {
            // from documentation: ROR by n where n is greater than 32 will give the same
            // result and carry out as ROR by n-32; therefore repeatedly y subtract 32 from n until the amount is
            // in the range 1 to 32
            let mut new_shift_amount = shift_amount;

            if shift_amount > 32 {
                new_shift_amount %= 32;

                // if modulo operation yields 0 it means that shift_amount was a multiple of 32
                // so we should do ROR#32
                if new_shift_amount == 0 {
                    new_shift_amount = 32;
                }
            }

            match new_shift_amount {
                // ROR#0 is used to encode RRX (appending C to the left and shift right by 1)
                0 => {
                    let old_carry = carry as u32;

                    ArithmeticOpResult {
                        result: (rm >> 1) | (old_carry << 31),
                        carry: rm.get_bit(0),
                        ..Default::default()
                    }
                }

                // ROR#1..31: normal rotate right
                1..=31 => ArithmeticOpResult {
                    result: rm.rotate_right(new_shift_amount),
                    carry: rm.get_bit((new_shift_amount - 1).try_into().unwrap()),
                    ..Default::default()
                },

                // ROR#32 doesn't change rm but sets carry to bit 31 of rm
                32 => ArithmeticOpResult {
                    result: rm,
                    carry: rm.get_bit(31),
                    ..Default::default()
                },

                // ROR#i with i > 32 is the same of ROR#n where n = i % 32
                _ => unreachable!(),
            }
        }
    }
}

/// Represents the kind of PSR operation
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PsrOpKind {
    /// MSR operation (transfer PSR contents to a register)
    Mrs { destination_register: u32 },
    /// MSR operation (transfer register contents to PSR)
    Msr { source_register: u32 },
    /// MSR flags operation (transfer register contents or immediate value to PSR flag bits only)
    MsrFlg { operand: AluSecondOperandInfo },
}

impl From<u32> for PsrOpKind {
    fn from(op_code: u32) -> Self {
        if op_code.get_bits(23..=27) == 0b0_0010
            && op_code.get_bits(16..=21) == 0b00_1111
            && op_code.get_bits(0..=11) == 0b0000_0000_0000
        {
            Self::Mrs {
                destination_register: op_code.get_bits(12..=15),
            }
        } else if op_code.get_bits(23..=27) == 0b00010
            && op_code.get_bits(12..=21) == 0b10_1001_1111
            && op_code.get_bits(4..=11) == 0b0000_0000
        {
            Self::Msr {
                source_register: op_code.get_bits(0..=3),
            }
        } else if op_code.get_bits(26..=27) == 0b00
            && op_code.get_bits(23..=24) == 0b10
            && op_code.get_bits(12..=21) == 0b10_1000_1111
        {
            Self::MsrFlg {
                operand: if op_code.get_bit(25) {
                    AluSecondOperandInfo::Immediate {
                        base: op_code.get_bits(0..=7),
                        shift: op_code.get_bits(8..=11) * 2,
                    }
                } else {
                    AluSecondOperandInfo::Register {
                        shift_op: ShiftOperator::Immediate(0),
                        shift_kind: ShiftKind::Lsl,
                        register: op_code.get_bits(0..=3),
                    }
                },
            }
        } else {
            unreachable!()
        }
    }
}

/// Represents the kind of PSR register to user
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PsrKind {
    Cpsr,
    Spsr,
}

impl From<bool> for PsrKind {
    fn from(value: bool) -> Self {
        if value {
            Self::Spsr
        } else {
            Self::Cpsr
        }
    }
}

impl std::fmt::Display for PsrKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cpsr => write!(f, "CPSR"),
            Self::Spsr => write!(f, "SPSR"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ShiftOperator {
    Immediate(u32),
    Register(u32),
}

impl std::fmt::Display for ShiftOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Immediate(value) => write!(f, "#{value}"),
            Self::Register(register) => write!(f, "R{register}"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AluSecondOperandInfo {
    Register {
        shift_op: ShiftOperator,
        shift_kind: ShiftKind,
        register: u32,
    },
    Immediate {
        base: u32,
        shift: u32,
    },
}

impl std::fmt::Display for AluSecondOperandInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Register {
                shift_op,
                shift_kind,
                register,
            } => {
                if let ShiftOperator::Immediate(shift) = shift_op {
                    if shift == 0 {
                        return if shift_kind == ShiftKind::Lsl {
                            write!(f, "R{register}")
                        } else if shift_kind == ShiftKind::Ror {
                            write!(f, "R{register}, RRX")
                        } else {
                            write!(f, "R{register}, {shift_kind} #32")
                        };
                    }
                }

                write!(f, "R{register}, {shift_kind} {shift_op}")
            }
            Self::Immediate { base, shift } => {
                write!(f, "#{}", base.rotate_right(shift))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_logical_instruction() {
        let alu_op_code = 9;
        let instruction_kind = ArmModeAluInstr::from(alu_op_code).kind();

        assert_eq!(instruction_kind, AIKind::Logical);
    }

    #[test]
    fn test_arithmetic_instruction() {
        let alu_op_code = 2;
        let instruction_kind = ArmModeAluInstr::from(alu_op_code).kind();

        assert_eq!(instruction_kind, AIKind::Arithmetic);
    }
}
