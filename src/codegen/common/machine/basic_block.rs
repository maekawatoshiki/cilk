// use super::{module::*, opcode::*, value::*};
use crate::codegen::arch::machine::inst::*;
use crate::codegen::arch::machine::register::{
    PhysRegSet, RegisterId, TargetRegisterTrait, VirtOrPhys, CALLEE_SAVED_REGS,
};
use crate::traits::basic_block::*;
use id_arena::*;
use rustc_hash::{FxHashMap, FxHashSet};
use std::cell::{Ref, RefCell, RefMut};

pub type MachineBasicBlockId = Id<MachineBasicBlock>;

#[derive(Debug, Clone)]
pub struct MachineBasicBlocks {
    pub arena: Arena<MachineBasicBlock>,
    pub order: Vec<MachineBasicBlockId>,
    pub liveness: FxHashMap<MachineBasicBlockId, LivenessInfo>,
}

#[derive(Clone, Debug)]
pub struct MachineBasicBlock {
    /// Predecessors
    pub pred: FxHashSet<MachineBasicBlockId>,

    /// Successors
    pub succ: FxHashSet<MachineBasicBlockId>,

    /// Instruction list
    pub iseq: RefCell<Vec<MachineInstId>>,
}

#[derive(Clone, Debug)]
pub struct LivenessInfo {
    pub phys_def: PhysRegSet,
    pub def: FxHashSet<RegisterId>,
    pub live_in: FxHashSet<RegisterId>,
    pub live_out: FxHashSet<RegisterId>,
    pub has_call: bool,
}

impl BasicBlockTrait for MachineBasicBlock {
    fn get_preds(&self) -> &FxHashSet<Id<Self>> {
        &self.pred
    }

    fn get_succs(&self) -> &FxHashSet<Id<Self>> {
        &self.succ
    }
}

impl BasicBlocksTrait for MachineBasicBlocks {
    type BB = MachineBasicBlock;

    fn get_arena(&self) -> &Arena<Self::BB> {
        &self.arena
    }

    fn get_order(&self) -> &Vec<Id<Self::BB>> {
        &self.order
    }
}

impl MachineBasicBlocks {
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
            order: vec![],
            liveness: FxHashMap::default(),
        }
    }

    pub fn id_and_block(&self) -> BasicBlocksIter<MachineBasicBlocks> {
        BasicBlocksIter::new(self)
    }

    pub fn get_def_phys_regs(&self) -> PhysRegSet {
        let mut set = PhysRegSet::new();
        for (id, _) in self.id_and_block() {
            set = set | self.liveness[&id].phys_def.clone();
        }
        set
    }

    pub fn merge(&mut self, dst: &MachineBasicBlockId, src: &MachineBasicBlockId) {
        let src_block = ::std::mem::replace(&mut self.arena[*src], MachineBasicBlock::new());
        let MachineBasicBlock {
            succ: src_succ,
            iseq: src_iseq,
            ..
        } = src_block;
        for &succ in &src_succ {
            self.arena[succ].pred.remove(src);
            self.arena[succ].pred.insert(*dst);
        }
        let dst_block = &mut self.arena[*dst];
        dst_block.succ = src_succ;
        dst_block
            .iseq
            .borrow_mut()
            .append(&mut src_iseq.borrow_mut());
        let src_liveness = self.liveness[src].clone();
        self.liveness.get_mut(dst).unwrap().merge(src_liveness);
        self.order.retain(|bb| bb != src);
    }

    pub fn remove_reg_from_live_info(&mut self, r: &RegisterId) {
        for id in &self.order {
            let liveness_ref = self.liveness.get_mut(id).unwrap();
            liveness_ref.def.remove(&r);
            liveness_ref.live_in.remove(&r);
            liveness_ref.live_out.remove(&r);
        }
    }
}

impl MachineBasicBlock {
    pub fn new() -> Self {
        Self {
            iseq: RefCell::new(vec![]),
            pred: FxHashSet::default(),
            succ: FxHashSet::default(),
        }
    }

    pub fn iseq_ref(&self) -> Ref<Vec<MachineInstId>> {
        self.iseq.borrow()
    }

    pub fn iseq_ref_mut(&self) -> RefMut<Vec<MachineInstId>> {
        self.iseq.borrow_mut()
    }

    pub fn find_inst_pos(&self, id2find: MachineInstId) -> Option<usize> {
        self.iseq_ref()
            .iter()
            .enumerate()
            .find(|(_, id)| *id == &id2find)
            .map(|(i, _)| i)
    }
}

impl LivenessInfo {
    pub fn new() -> Self {
        Self {
            phys_def: PhysRegSet::new(),
            def: FxHashSet::default(),
            live_in: FxHashSet::default(),
            live_out: FxHashSet::default(),
            has_call: false,
        }
    }

    pub fn add_phys_def<T: TargetRegisterTrait>(&mut self, r: T) {
        self.phys_def.unite(&r.regs_sharing_same_register_file());
    }

    /// Add def.
    /// If ``self.def`` has ``reg``'s super physical register, we don't.
    /// If ``self.def`` has ``reg``'s sub physical register, remove it and add ``reg``.
    pub fn add_def(&mut self, reg: RegisterId) -> bool {
        if reg.is_virt_reg() {
            return self.def.insert(reg);
        }

        let p = reg.as_phys_reg();
        let mut cur = p;

        if CALLEE_SAVED_REGS.with(|regs| regs.has(p)) {
            return false;
        }

        if self.def.contains(&reg) {
            return false;
        }

        while let Some(sub) = cur.sub_reg() {
            cur = sub;
            self.def.retain(|k| k.kind != VirtOrPhys::Phys(cur));
        }

        cur = p;

        while let Some(sub) = cur.super_reg() {
            cur = sub;
            if self.def.iter().any(|k| k.kind == VirtOrPhys::Phys(cur)) {
                return false;
            }
        }

        self.def.insert(reg)
    }

    /// Add livein.
    /// If ``self.live_in`` has ``reg``'s super physical register, we don't.
    /// If ``self.live_in`` has ``reg``'s sub physical register, remove it and add ``reg``.
    pub fn add_live_in(&mut self, reg: RegisterId) -> bool {
        if reg.is_virt_reg() {
            return self.live_in.insert(reg);
        }

        let p = reg.as_phys_reg();
        let mut cur = p;

        if CALLEE_SAVED_REGS.with(|regs| regs.has(p)) {
            return false;
        }

        if self.live_in.contains(&reg) {
            return false;
        }

        while let Some(sub) = cur.sub_reg() {
            cur = sub;
            self.live_in.retain(|k| k.kind != VirtOrPhys::Phys(cur));
        }

        cur = p;

        while let Some(sub) = cur.super_reg() {
            cur = sub;
            if self.live_in.iter().any(|k| k.kind == VirtOrPhys::Phys(cur)) {
                return false;
            }
        }

        self.live_in.insert(reg)
    }

    /// Add liveout.
    /// If ``self.live_out`` has ``reg``'s super physical register, we don't.
    /// If ``self.live_out`` has ``reg``'s sub physical register, remove it and add ``reg``.
    pub fn add_live_out(&mut self, reg: RegisterId) -> bool {
        if reg.is_virt_reg() {
            return self.live_out.insert(reg);
        }

        let p = reg.as_phys_reg();
        let mut cur = p;

        if CALLEE_SAVED_REGS.with(|regs| regs.has(p)) {
            return false;
        }

        if self.live_out.contains(&reg) {
            return false;
        }

        while let Some(sub) = cur.sub_reg() {
            cur = sub;
            self.live_out.retain(|k| k.kind != VirtOrPhys::Phys(cur));
        }

        cur = p;

        while let Some(sub) = cur.super_reg() {
            cur = sub;
            if self
                .live_out
                .iter()
                .any(|k| k.kind == VirtOrPhys::Phys(cur))
            {
                return false;
            }
        }

        self.live_out.insert(reg)
    }

    // merge src into self
    pub fn merge(&mut self, src: LivenessInfo) {
        for def in src.def {
            self.add_def(def);
        }
        for live_in in src.live_in {
            self.add_live_in(live_in);
        }
        for live_out in src.live_out {
            self.add_live_out(live_out);
        }
        self.has_call |= src.has_call;
        self.phys_def.unite(&src.phys_def);
    }

    pub fn clear(&mut self) {
        self.phys_def = PhysRegSet::new();
        self.has_call = false;
        self.def.clear();
        self.live_in.clear();
        self.live_out.clear();
    }
}
