use crate::ir::{
    function::Function,
    module::Module,
    opcode::{InstOperand, Instruction, InstructionId, Opcode, Operand},
    value::{InstructionValue, Value},
};

pub struct DeadCodeElimination {}

struct DeadCodeEliminationOnFunction<'a> {
    func: &'a mut Function,
}

impl DeadCodeElimination {
    pub fn new() -> Self {
        Self {}
    }

    pub fn run_on_module(&mut self, module: &mut Module) {
        for (_, func) in &mut module.functions {
            if func.is_internal || func.is_empty() {
                continue;
            }

            DeadCodeEliminationOnFunction { func }.run()
        }
    }
}

impl<'a> DeadCodeEliminationOnFunction<'a> {
    pub fn run(self) {
        let mut elimination_list = vec![];
        let mut worklist = vec![];

        for &block_id in &self.func.basic_blocks.order {
            let block = &self.func.basic_blocks.arena[block_id];
            for &inst_id in &*block.iseq_ref() {
                let inst = &self.func.inst_table[inst_id];
                Self::check_if_elimination_possible(inst, &mut elimination_list, &mut worklist);
            }
        }

        while let Some(inst_id) = worklist.pop() {
            let inst = &self.func.inst_table[inst_id];
            Self::check_if_elimination_possible(inst, &mut elimination_list, &mut worklist);
        }

        for inst_id in elimination_list {
            self.func.remove_inst(inst_id)
        }
    }

    fn check_if_elimination_possible(
        inst: &Instruction,
        elimination_list: &mut Vec<InstructionId>,
        worklist: &mut Vec<InstructionId>,
    ) {
        let dont_eliminate =
            matches!(inst.opcode, Opcode::Store | Opcode::Call) || inst.opcode.is_terminator();
        if dont_eliminate {
            return;
        }
        // no users then eliminate
        if inst.users.borrow().len() == 0 {
            elimination_list.push(inst.id.unwrap());
            if !matches!(inst.operand, InstOperand::None) {
                for op in inst.operand.args() {
                    if let Value::Instruction(InstructionValue { id, .. }) = op {
                        worklist.push(*id)
                    }
                }
            } else {
                for op in &inst.operands {
                    if let Operand::Value(Value::Instruction(InstructionValue { id, .. })) = op {
                        worklist.push(*id)
                    }
                }
            }
        }
    }
}
