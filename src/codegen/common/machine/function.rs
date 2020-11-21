// use super::super::dag::function::*;
use super::super::machine::register::*;
use crate::codegen::arch::machine::{frame_object::*, inst::*};
use crate::codegen::common::machine::const_data::ConstDataArena;
use crate::codegen::common::{dag::function::*, machine::basic_block::*};
use crate::ir::types::*;
use crate::traits::function::FunctionTrait;
use id_arena::*;
use std::cell::Ref;
use std::fmt;
use std::ops::{Index, IndexMut};

pub type MachineFunctionId = Id<MachineFunction>;

// TODO: Documents in detail
pub struct MachineFunction {
    /// Id
    pub id: Option<MachineFunctionId>,

    /// Function name
    pub name: String,

    /// Function type
    pub ty: Type,

    /// Include basic blocks and instruction arena
    pub body: MachineFunctionBody,

    /// True if internal function
    pub is_internal: bool,

    /// Local variables info
    pub local_mgr: LocalVariables,

    pub regs_info: RegistersInfo,

    pub frame_objects: Option<FrameObjectsInfo>,

    pub const_data: ConstDataArena,

    pub types: Types,
}

#[derive(Clone, Debug)]
pub struct InstructionArena {
    pub arena: Arena<MachineInst>,
}

#[derive(Clone, Debug)]
pub struct MachineFunctionBody {
    pub inst_arena: InstructionArena,
    pub basic_blocks: MachineBasicBlocks,
}

pub struct MBBIter<'a> {
    inst_arena: &'a InstructionArena,
    basic_blocks: &'a MachineBasicBlocks,
    nth: usize,
}

pub struct InstIter<'a> {
    inst_arena: &'a InstructionArena,
    inst_id_seq: Ref<'a, Vec<MachineInstId>>,
    nth: usize,
}

impl MachineFunctionBody {
    pub fn mbb_iter<'a>(&'a self) -> MBBIter<'a> {
        MBBIter {
            inst_arena: &self.inst_arena,
            basic_blocks: &self.basic_blocks,
            nth: 0,
        }
    }

    pub fn has_call(&self) -> bool {
        self.basic_blocks
            .id_and_block()
            .any(|(id, _)| self.basic_blocks.liveness[&id].has_call)
    }

    pub fn appeared_phys_regs(&self) -> PhysRegSet {
        let mut set = PhysRegSet::new();
        for (id, _) in self.basic_blocks.id_and_block() {
            set.unite(&self.basic_blocks.liveness[&id].phys_def);
        }
        set
    }
}

impl FunctionTrait for MachineFunction {
    type BBS = MachineBasicBlocks;

    fn get_basic_blocks(&self) -> &Self::BBS {
        &self.body.basic_blocks
    }
}

impl<'a> Iterator for MBBIter<'a> {
    type Item = (MachineBasicBlockId, &'a MachineBasicBlock, InstIter<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        self.nth += 1;
        let id = *self.basic_blocks.order.get(self.nth - 1)?;
        Some((
            id,
            &self.basic_blocks.arena[id],
            InstIter {
                inst_arena: self.inst_arena,
                inst_id_seq: self.basic_blocks.arena[id].iseq_ref(),
                nth: 0,
            },
        ))
    }
}

impl<'a> Iterator for InstIter<'a> {
    type Item = (MachineInstId, &'a MachineInst);

    fn next(&mut self) -> Option<Self::Item> {
        self.nth += 1;
        let id = *self.inst_id_seq.get(self.nth - 1)?;
        Some((id, &self.inst_arena[id]))
    }
}

impl MachineFunction {
    pub fn new(
        f: DAGFunction,
        basic_blocks: MachineBasicBlocks,
        inst_arena: InstructionArena,
    ) -> Self {
        Self {
            id: None,
            is_internal: f.is_internal,
            name: f.name,
            ty: f.ty,
            body: MachineFunctionBody {
                inst_arena,
                basic_blocks,
            },
            local_mgr: f.local_vars,
            regs_info: f.regs,
            frame_objects: None,
            const_data: ConstDataArena::new(),
            types: f.types.clone(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.get_basic_blocks().order.len() == 0
    }

    pub fn alloc_inst(&mut self, inst: MachineInst) -> MachineInstId {
        self.body.inst_arena.alloc(&mut self.regs_info, inst)
    }

    pub fn find_inst_pos(&self, inst_id: MachineInstId) -> Option<(MachineBasicBlockId, usize)> {
        let parent = self.body.inst_arena[inst_id].parent;
        self.body.basic_blocks.arena[parent]
            .find_inst_pos(inst_id)
            .map(|pos| (parent, pos))
    }

    pub fn remove_inst(&self, inst_id: MachineInstId) {
        let (bb_id, pos) = self.find_inst_pos(inst_id).unwrap();
        // TODO: refine code
        for d in &self.body.inst_arena[inst_id].collect_defined_regs() {
            self.regs_info.arena_ref_mut()[d.id].remove_def(inst_id);
        }
        for u in &self.body.inst_arena[inst_id].collect_used_regs() {
            self.regs_info.arena_ref_mut()[u.id].remove_use(inst_id);
        }
        self.body.basic_blocks.arena[bb_id]
            .iseq_ref_mut()
            .remove(pos);
    }

    pub fn get_entry_bb(&self) -> Option<&MachineBasicBlockId> {
        self.body.basic_blocks.order.get(0)
    }

    // for more precise information (of type) than Debug trait
    pub fn debug(&self, f: &mut fmt::Formatter, tys: &Types) -> fmt::Result {
        writeln!(
            f,
            "MachineFunction(name: {}, ty: {}):",
            self.name,
            tys.to_string(self.ty)
        )?;

        let mut idx = 0;
        for (id, bb, iiter) in self.body.mbb_iter() {
            writeln!(f, "MachineBasicBlock #{} ({:?})", id.index(), bb)?;
            for (id, inst) in iiter {
                write!(f, "{: ^4}({: ^4}): ", idx, id.index())?;
                inst.debug(tys, f)?;
                writeln!(f)?;
                idx += 1;
            }
        }

        fmt::Result::Ok(())
    }
}

impl fmt::Debug for MachineFunction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "MachineFunction(name: {}, ty: {:?}):",
            self.name, self.ty
        )?;

        let mut idx = 0;
        for (id, bb, iiter) in self.body.mbb_iter() {
            writeln!(f, "MachineBasicBlock #{} ({:?})", id.index(), bb)?;
            for (id, inst) in iiter {
                writeln!(f, "{: ^4}({: ^4}): {:?}", idx, id.index(), inst)?;
                idx += 1;
            }
        }

        fmt::Result::Ok(())
    }
}

impl InstructionArena {
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
        }
    }

    pub fn alloc(&mut self, regs_info: &RegistersInfo, mi: MachineInst) -> Id<MachineInst> {
        let mi_id = self.arena.alloc(mi);
        let mi = &mut self.arena[mi_id];
        mi.set_id(regs_info, mi_id);
        mi_id
    }
}

impl Index<MachineInstId> for InstructionArena {
    type Output = MachineInst;

    fn index(&self, idx: MachineInstId) -> &Self::Output {
        &self.arena[idx]
    }
}

impl IndexMut<MachineInstId> for InstructionArena {
    fn index_mut(&mut self, idx: MachineInstId) -> &mut Self::Output {
        &mut self.arena[idx]
    }
}
