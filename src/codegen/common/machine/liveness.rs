use crate::codegen::arch::machine::{inst::*, register::*};
use crate::codegen::common::machine::{basic_block::*, function::*};
use crate::util::allocator::{Raw, RawAllocator};
use rustc_hash::FxHashMap;
use std::cmp::Ordering;
use std::fmt;

const IDX_STEP: usize = 16;

pub struct LiveRegMatrix {
    pub virt_regs: FxHashMap<VirtReg, RegisterId>,
    pub id2pp: FxHashMap<MachineInstId, ProgramPoint>,
    pub virt_reg_interval: VirtRegInterval,
    pub phys_reg_range: PhysRegRange,
    pub program_points: ProgramPoints,
}

pub struct PhysRegRange(FxHashMap<RegKey, LiveRange>);
pub struct VirtRegInterval(FxHashMap<VirtReg, LiveInterval>);

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct RegKey(usize);

#[derive(Debug, Clone)]
pub struct LiveInterval {
    pub vreg: VirtReg,
    pub reg: Option<PhysReg>,
    pub range: LiveRange,
    pub spill_weight: f32,
    pub is_spillable: bool,
}

#[derive(Debug, Clone)]
pub struct LiveRange {
    // TODO: segments should be sorted by LiveSegment.start
    pub segments: Vec<LiveSegment>,
}

#[derive(Debug, Clone, Copy)]
pub struct LiveSegment {
    pub start: ProgramPoint,
    pub end: ProgramPoint,
}

#[derive(Clone, Copy)]
pub struct ProgramPoint {
    pub base: Raw<ProgramPointBase>,
}

#[derive(Debug, Clone, Copy)]
pub struct ProgramPointBase {
    pub prev: Option<Raw<ProgramPointBase>>,
    pub next: Option<Raw<ProgramPointBase>>,
    bb: usize,
    idx: usize,
}

pub struct ProgramPoints {
    allocator: RawAllocator<ProgramPointBase>,
}

impl LiveRegMatrix {
    pub fn new(
        virt_regs: FxHashMap<VirtReg, RegisterId>,
        id2pp: FxHashMap<MachineInstId, ProgramPoint>,
        virt_reg_interval: VirtRegInterval,
        phys_reg_range: PhysRegRange,
        program_points: ProgramPoints,
    ) -> Self {
        Self {
            virt_regs,
            id2pp,
            virt_reg_interval,
            phys_reg_range,
            program_points,
        }
    }

    // pub fn contains_vreg_entity(&self, vreg: &MachineRegister) -> bool {
    //     self.virt_regs.contains_key(&vreg.get_vreg())
    // }

    pub fn add_virt_reg(&mut self, reg: RegisterId) {
        self.virt_regs.insert(reg.as_virt_reg(), reg);
    }

    pub fn add_live_interval(&mut self, vreg: VirtReg, range: LiveRange) -> &mut LiveInterval {
        self.virt_reg_interval.add(vreg, range)
    }

    pub fn get_program_point(&self, id: MachineInstId) -> Option<ProgramPoint> {
        self.id2pp.get(&id).map(|x| *x)
    }

    /// Return false if it's legal to allocate reg for vreg
    pub fn interferes(&self, vreg: VirtReg, reg: PhysReg) -> bool {
        let r1 = match self.phys_reg_range.get(reg) {
            Some(r1) => r1,
            None => return false,
        };
        let r2 = &self.virt_reg_interval.get(&vreg).unwrap().range;

        r1.interferes(r2)
    }

    /// Return false if it's legal to allocate reg for vreg
    pub fn interferes_virt_regs(&self, vreg1: VirtReg, vreg2: VirtReg) -> bool {
        let r1 = &self.virt_reg_interval.get(&vreg1).unwrap().range;
        let r2 = &self.virt_reg_interval.get(&vreg2).unwrap().range;
        r1.interferes(r2)
    }

    pub fn interferes_with_range(&self, vreg: VirtReg, range: &LiveRange) -> bool {
        match self.virt_reg_interval.get(&vreg) {
            Some(interval) => range.interferes(&interval.range),
            None => false,
        }
    }

    pub fn interferes_phys_with_range(&self, reg: PhysReg, range: &LiveRange) -> bool {
        match self.phys_reg_range.get(reg.into()) {
            Some(range2) => range.interferes(range2),
            None => false,
        }
    }

    pub fn collect_interfering_assigned_regs(&self, vreg: VirtReg) -> Vec<VirtReg> {
        let mut interferings = vec![];
        let i = self.virt_reg_interval.get(&vreg).unwrap();

        for (cur_vreg, interval) in self.virt_reg_interval.inner() {
            if vreg == *cur_vreg {
                continue;
            }

            if interval.interferes(i) && interval.reg.is_some() {
                interferings.push(*cur_vreg);
            }
        }

        interferings
    }

    // pub fn pick_assigned_and_longest_lived_vreg(&self, vregs: &[VirtReg]) -> Option<VirtReg> {
    //     let mut longest: Option<(ProgramPoint, VirtReg)> = None;
    //
    //     for vreg in vregs {
    //         match longest {
    //             Some((ref mut endpp1, ref mut vreg1)) => {
    //                 let interval2 = self.virt_reg_interval.get(vreg).unwrap();
    //                 if interval2.reg.is_none() {
    //                     continue;
    //                 }
    //                 let endpp2 = interval2.end_point().unwrap();
    //                 if *endpp1 < endpp2 {
    //                     *endpp1 = endpp2;
    //                     *vreg1 = *vreg
    //                 }
    //             }
    //             None => {
    //                 let interval = self.virt_reg_interval.get(vreg).unwrap();
    //                 if interval.reg.is_none() {
    //                     continue;
    //                 }
    //                 longest = Some((interval.end_point().unwrap(), *vreg))
    //             }
    //         }
    //     }
    //
    //     longest.and_then(|(_, vreg)| Some(vreg))
    // }

    pub fn assign_reg(&mut self, vreg: VirtReg, reg: PhysReg) {
        // assign reg to vreg
        self.virt_reg_interval.get_mut(&vreg).unwrap().reg = Some(reg);

        let range = self.virt_reg_interval.get(&vreg).unwrap().range.clone();
        self.phys_reg_range.get_or_create(reg).unite_range(range);
    }

    pub fn unassign_reg(&mut self, vreg: VirtReg) -> Option<PhysReg> {
        let maybe_reg = &mut self.virt_reg_interval.get_mut(&vreg).unwrap().reg;

        if maybe_reg.is_none() {
            return None;
        }

        let reg = maybe_reg.unwrap();
        // unassign physical register
        *maybe_reg = None;

        let range = &self.virt_reg_interval.get(&vreg).unwrap().range;
        self.phys_reg_range.get_or_create(reg).remove_range(range);
        assert!(!self.phys_reg_range.get_or_create(reg).interferes(range));

        Some(reg)
    }

    /// v2 merges into v1 and remove v2 from matrix
    pub fn merge_virt_regs(&mut self, regs_info: &RegistersInfo, v1: VirtReg, v2: VirtReg) {
        let v2_e = self.virt_regs.remove(&v2).unwrap();
        let v1_e = *self.virt_regs.get(&v1).unwrap();
        let mut arena = regs_info.arena_ref_mut();
        for use_ in arena[v2_e].uses.clone() {
            arena[v1_e].add_use(use_)
        }
        for def in arena[v2_e].defs.clone() {
            arena[v1_e].add_def(def)
        }
        let v2_i = self.virt_reg_interval.remove(&v2).unwrap();
        let v1_i = self.virt_reg_interval.get_mut(&v1).unwrap();
        v1_i.range.unite_range(v2_i.range);
    }

    /// r2 merges into r1 and remove r2 from matrix
    pub fn merge_regs(&mut self, r1: PhysReg, r2: VirtReg) {
        self.virt_regs.remove(&r2);
        let r2_i = self.virt_reg_interval.remove(&r2).unwrap();
        let r1_i = self.phys_reg_range.get_mut(r1).unwrap();
        r1_i.unite_range(r2_i.range);
    }

    pub fn collect_virt_regs(&self) -> Vec<VirtReg> {
        self.virt_reg_interval
            .inner()
            .iter()
            .map(|(vreg, _)| *vreg)
            .collect::<Vec<_>>()
    }

    pub fn get_entity_by_vreg(&self, vreg: VirtReg) -> Option<&RegisterId> {
        self.virt_regs.get(&vreg)
    }
}

impl PhysRegRange {
    pub fn get_or_create(&mut self, reg: PhysReg) -> &mut LiveRange {
        self.0.entry(reg.into()).or_insert(LiveRange::new_empty())
    }

    pub fn get(&self, reg: PhysReg) -> Option<&LiveRange> {
        self.0.get(&reg.into())
    }

    pub fn get_mut(&mut self, reg: PhysReg) -> Option<&mut LiveRange> {
        self.0.get_mut(&reg.into())
    }
}

impl VirtRegInterval {
    pub fn inner(&self) -> &FxHashMap<VirtReg, LiveInterval> {
        &self.0
    }

    pub fn inner_mut(&mut self) -> &mut FxHashMap<VirtReg, LiveInterval> {
        &mut self.0
    }

    pub fn get(&self, vreg: &VirtReg) -> Option<&LiveInterval> {
        self.0.get(vreg)
    }

    pub fn get_mut(&mut self, vreg: &VirtReg) -> Option<&mut LiveInterval> {
        self.0.get_mut(vreg)
    }

    pub fn remove(&mut self, vreg: &VirtReg) -> Option<LiveInterval> {
        self.0.remove(vreg)
    }

    pub fn add(&mut self, vreg: VirtReg, range: LiveRange) -> &mut LiveInterval {
        self.0.entry(vreg).or_insert(LiveInterval::new(vreg, range))
    }

    pub fn collect_virt_regs(&self) -> Vec<VirtReg> {
        self.0.iter().map(|(vreg, _)| *vreg).collect()
    }
}

impl From<PhysReg> for RegKey {
    fn from(r: PhysReg) -> Self {
        RegKey(
            r.retrieve() - r.reg_class() as usize
                + r.reg_class().register_file_base_class() as usize,
        )
    }
}

impl LiveInterval {
    pub fn new(vreg: VirtReg, range: LiveRange) -> Self {
        Self {
            vreg,
            range,
            reg: None,
            spill_weight: 0.0,
            is_spillable: true,
        }
    }

    pub fn interferes(&self, other: &LiveInterval) -> bool {
        self.range.interferes(&other.range)
    }

    pub fn start_point(&self) -> Option<ProgramPoint> {
        self.range.start_point()
    }

    pub fn end_point(&self) -> Option<ProgramPoint> {
        self.range.end_point()
    }

    pub fn end_point_mut(&mut self) -> Option<&mut ProgramPoint> {
        self.range.end_point_mut()
    }
}

impl LiveRange {
    pub fn new(segments: Vec<LiveSegment>) -> Self {
        Self { segments }
    }

    pub fn new_empty() -> Self {
        Self { segments: vec![] }
    }

    pub fn unused_range(&self, pps: &ProgramPoints) -> LiveRange {
        let mut r = LiveRange::new_empty();
        let mut last_pp = pps.lookup(0, 0).unwrap();
        for seg in &self.segments {
            r.add_segment(LiveSegment::new(last_pp, seg.start));
            last_pp = seg.end;
        }
        r
    }

    pub fn adjust_end_to_start(&mut self) {
        let start = match self.segments.iter().min_by(|x, y| x.start.cmp(&y.start)) {
            Some(seg) => seg.start,
            None => return,
        };
        self.segments = vec![LiveSegment::new(start, start)];
    }

    pub fn add_segment(&mut self, seg: LiveSegment) {
        let mut found = true;
        let pt = self
            .segments
            .binary_search_by(|s| s.start.cmp(&seg.start))
            .unwrap_or_else(|x| {
                found = false;
                x
            });

        fn arrange(range: &mut Vec<LiveSegment>, start: usize) {
            if start + 1 >= range.len() {
                return;
            }

            if range[start].end < range[start + 1].start {
                // no problem
            } else if range[start + 1].start <= range[start].end
                && range[start].end < range[start + 1].end
            {
                range[start].end = range[start + 1].end;
                range.remove(start + 1);
                return arrange(range, start);
            } else if range[start + 1].end <= range[start].end {
                range.remove(start + 1);
                return arrange(range, start);
            }

            arrange(range, start + 1);
        }

        if pt == 0 {
            self.segments.insert(pt, seg);
            return arrange(&mut self.segments, 0);
        }

        if found {
            if self.segments[pt].end < seg.end {
                self.segments[pt].end = seg.end;
                return arrange(&mut self.segments, pt);
            }
        } else {
            self.segments.insert(pt, seg);
            return arrange(&mut self.segments, pt - 1);
        }
    }

    pub fn unite_range(&mut self, range: LiveRange) {
        for seg in range.segments {
            self.add_segment(seg)
        }
    }

    pub fn remove_segment(&mut self, seg: &LiveSegment) {
        if self.segments.len() == 0 {
            return;
        }

        let mut found = true;
        let pt = self
            .segments
            .binary_search_by(|s| s.start.cmp(&seg.start))
            .unwrap_or_else(|x| {
                found = false;
                x
            });

        if !found && pt == 0 {
            if self.segments[0].start < seg.end {
                self.remove_segment(&LiveSegment::new(self.segments[0].start, seg.end));
            }
            return;
        }

        if found {
            if seg.end == self.segments[pt].end {
                self.segments.remove(pt);
            } else if seg.end < self.segments[pt].end {
                self.segments[pt].start = seg.end;
            } else if self.segments[pt].end < seg.end {
                self.segments.remove(pt);
                if pt + 1 >= self.segments.len() {
                    return;
                }
                if self.segments[pt + 1].start < seg.end {
                    self.remove_segment(&LiveSegment::new(self.segments[pt + 1].start, seg.end));
                }
            }
        } else {
            if self.segments[pt - 1].end <= seg.start {
                if pt >= self.segments.len() {
                    return;
                }
                if seg.start < self.segments[pt].start {
                    if self.segments[pt].start < seg.end {
                        if !self.interferes(&LiveRange::new(vec![*seg])) {
                            self.remove_segment(&LiveSegment::new(
                                self.segments[pt].start,
                                seg.end,
                            ));
                        }
                        return;
                    }
                }
                assert!(seg.end < self.segments[pt].start);
            } else if seg.end < self.segments[pt - 1].end {
                let end = self.segments[pt - 1].end;
                self.segments[pt - 1].end = seg.start;
                self.add_segment(LiveSegment::new(seg.end, end));
            } else if self.segments[pt - 1].end == seg.end {
                self.segments[pt - 1].end = seg.start;
            } else if self.segments[pt - 1].end < seg.end {
                assert!(seg.start < self.segments[pt - 1].end);
                let end = self.segments[pt - 1].end;
                self.segments[pt - 1].end = seg.start;
                self.remove_segment(&LiveSegment::new(end, seg.end));
            }
        }
    }

    pub fn remove_range(&mut self, range: &LiveRange) {
        for seg in &range.segments {
            self.remove_segment(seg)
        }
    }

    pub fn start_point(&self) -> Option<ProgramPoint> {
        self.segments
            .iter()
            .min_by(|x, y| x.start.cmp(&y.start))
            .and_then(|seg| Some(seg.start))
    }

    pub fn end_point(&self) -> Option<ProgramPoint> {
        self.segments
            .iter()
            .max_by(|x, y| x.end.cmp(&y.end))
            .and_then(|seg| Some(seg.end))
    }

    pub fn end_point_mut(&mut self) -> Option<&mut ProgramPoint> {
        self.segments
            .iter_mut()
            .max_by(|x, y| x.end.cmp(&y.end))
            .and_then(|seg| Some(&mut seg.end))
    }

    pub fn interferes(&self, other: &LiveRange) -> bool {
        for seg1 in &self.segments {
            for seg2 in &other.segments {
                if seg1.interferes(seg2) {
                    return true;
                }
            }
        }
        false
    }

    pub fn contains_point(&self, pp: &ProgramPoint) -> bool {
        self.segments.iter().any(|s| s.contains_point(pp))
    }

    pub fn find_nearest_starting_segment_mut(
        &mut self,
        pp: &ProgramPoint,
    ) -> Option<&mut LiveSegment> {
        for s in self.segments.iter_mut().rev() {
            if &s.start < pp {
                return Some(s);
            }
        }
        None
    }
}

impl LiveSegment {
    pub fn new(start: ProgramPoint, end: ProgramPoint) -> Self {
        Self { start, end }
    }

    pub fn interferes(&self, seg: &LiveSegment) -> bool {
        self.start < seg.end && self.end > seg.start
    }

    pub fn contains_point(&self, pp: &ProgramPoint) -> bool {
        self.start <= *pp && *pp < self.end
    }
}

impl ProgramPoints {
    pub fn new() -> Self {
        Self {
            allocator: RawAllocator::new(),
        }
    }

    pub fn new_program_point(&mut self, ppb: ProgramPointBase) -> ProgramPoint {
        let p = ProgramPoint::new(self.allocator.alloc(ppb));
        p
    }

    pub fn prev_of(&mut self, pp: ProgramPoint) -> ProgramPoint {
        let mut next: Raw<_> = pp.base;
        let prev = pp.base.prev;

        if prev.is_none() {
            unimplemented!()
        }

        let mut prev: Raw<_> = prev.unwrap();
        let one_block = next.bb() == prev.bb();

        if !one_block {
            let new_pp = self.new_program_point(ProgramPointBase::new(
                Some(prev),
                Some(next),
                prev.bb(),
                prev.idx() + IDX_STEP,
            ));
            prev.next = Some(new_pp.base);
            next.prev = Some(new_pp.base);
            return new_pp;
        }
        let one_block = next.bb() == prev.bb();

        if !one_block {
            let new_pp = self.new_program_point(ProgramPointBase::new(
                Some(prev),
                Some(next),
                prev.bb(),
                prev.idx() + IDX_STEP,
            ));
            prev.next = Some(new_pp.base);
            next.prev = Some(new_pp.base);
            return new_pp;
        }

        // need to renumber program points belonging to the same block as pp
        if next.idx() - prev.idx() < 2 {
            prev.renumber_in_bb();
            return self.prev_of(pp);
        }

        let new_pp = self.new_program_point(ProgramPointBase::new(
            Some(prev),
            Some(next),
            prev.bb(),
            (next.idx() + prev.idx()) / 2,
        ));
        prev.next = Some(new_pp.base);
        next.prev = Some(new_pp.base);

        new_pp
    }

    pub fn next_of(&mut self, pp: ProgramPoint) -> ProgramPoint {
        let mut prev: Raw<_> = pp.base;
        let next = pp.base.next;

        if next.is_none() {
            unimplemented!()
        }

        let mut next: Raw<_> = next.unwrap();
        let one_block = next.bb() == prev.bb();

        if !one_block {
            let new_pp = self.new_program_point(ProgramPointBase::new(
                Some(prev),
                Some(next),
                prev.bb(),
                prev.idx() + IDX_STEP,
            ));
            prev.next = Some(new_pp.base);
            next.prev = Some(new_pp.base);
            return new_pp;
        }

        // need to renumber program points belonging to the same block as that of pp
        if next.idx() - prev.idx() < 2 {
            prev.renumber_in_bb();
            return self.next_of(pp);
        }

        let new_pp = self.new_program_point(ProgramPointBase::new(
            Some(prev),
            Some(next),
            prev.bb(),
            (prev.idx() + next.idx()) / 2,
        ));
        next.prev = Some(new_pp.base);
        prev.next = Some(new_pp.base);

        new_pp
    }

    pub fn lookup(&self, bb: usize, idx: usize) -> Option<ProgramPoint> {
        self.allocator
            .allocated_ref()
            .iter()
            .find(|pp| pp.bb() == bb && pp.idx() == idx)
            .map(|&ppb| ProgramPoint::new(ppb))
    }
}

impl ProgramPoint {
    pub fn new(base: Raw<ProgramPointBase>) -> Self {
        Self { base }
    }

    pub fn set_prev(mut self, pp: Option<ProgramPoint>) -> Self {
        some_then!(mut pp, pp, {
            pp.base.next = Some(self.base);
            self.base.prev = Some(pp.base)
        });
        self
    }

    pub fn idx(&self) -> usize {
        self.base.idx()
    }

    pub fn bb(&self) -> usize {
        self.base.bb()
    }
}

impl ProgramPointBase {
    pub fn new(
        prev: Option<Raw<ProgramPointBase>>,
        next: Option<Raw<ProgramPointBase>>,
        bb: usize,
        idx: usize,
    ) -> Self {
        Self {
            prev,
            next,
            bb,
            idx,
        }
    }

    pub fn idx(&self) -> usize {
        self.idx
    }

    pub fn bb(&self) -> usize {
        self.bb
    }

    // pub fn find_bb_start(&self) -> ProgramPointBase {
    //     let mut cur = *self;
    //     while let Some(prev) = cur.prev {
    //         if cur.bb() != prev.bb() {
    //             return cur;
    //         }
    //         cur = *prev;
    //     }
    //     cur
    // }

    pub fn renumber_in_bb(&self) {
        // panic!();
        let mut cur = *self;
        while let Some(mut next) = cur.next {
            if cur.bb != next.bb {
                break;
            }
            (*next).idx += IDX_STEP;
            cur = *next;
        }
    }
}

impl Ord for ProgramPoint {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.bb() < other.bb() {
            return Ordering::Less;
        }

        if self.bb() > other.bb() {
            return Ordering::Greater;
        }

        self.idx().cmp(&other.idx())
    }
}

impl Eq for ProgramPoint {}

impl fmt::Debug for ProgramPoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.bb(), self.idx())
    }
}

impl PartialOrd for ProgramPoint {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ProgramPoint {
    fn eq(&self, other: &Self) -> bool {
        self.bb() == other.bb() && self.idx() == other.idx()
    }
}

pub struct LivenessAnalysis {}

impl LivenessAnalysis {
    pub fn new() -> Self {
        Self {}
    }

    // pub fn analyze_module(&mut self, module: &mut MachineModule) {
    //     for (_, func) in &module.functions {
    //         self.analyze_function(func);
    //     }
    // }

    pub fn analyze_function(&mut self, cur_func: &mut MachineFunction) -> LiveRegMatrix {
        // self.clear_liveness_info(cur_func);
        let mut liveness = FxHashMap::default();
        self.set_def(cur_func, &mut liveness);
        self.visit(cur_func, &mut liveness);
        cur_func.body.basic_blocks.liveness = liveness;
        self.construct_live_reg_matrix(cur_func)
    }

    // fn clear_liveness_info(&mut self, cur_func: &MachineFunction) {
    //     for (_, bb) in &cur_func.body.basic_blocks.arena {
    //         bb.liveness_ref_mut().clear();
    //     }
    // }

    fn set_def(
        &mut self,
        cur_func: &MachineFunction,
        liveness: &mut FxHashMap<MachineBasicBlockId, LivenessInfo>,
    ) {
        for (id, _, iiter) in cur_func.body.mbb_iter() {
            let mut l = LivenessInfo::new();
            for (_, inst) in iiter {
                self.set_def_on_inst(&mut l, inst);
            }
            liveness.insert(id, l);
        }
    }

    fn set_def_on_inst(&mut self, liveness: &mut LivenessInfo, inst: &MachineInst) {
        liveness.has_call |= inst.opcode == MachineOpcode::CALL;
        for &reg in &inst.def {
            liveness.add_def(reg.id);
        }
        for &reg in &inst.imp_def {
            liveness.add_def(reg.id);
        }
    }

    fn visit(
        &mut self,
        cur_func: &MachineFunction,
        liveness: &mut FxHashMap<MachineBasicBlockId, LivenessInfo>,
    ) {
        for (bb_id, _, iiter) in cur_func.body.mbb_iter() {
            for (_, inst) in iiter {
                self.visit_inst(cur_func, bb_id, inst, liveness);
            }
        }
    }

    fn visit_inst(
        &mut self,
        cur_func: &MachineFunction,
        bb: MachineBasicBlockId,
        inst: &MachineInst,
        liveness: &mut FxHashMap<MachineBasicBlockId, LivenessInfo>,
    ) {
        for operand in &inst.operand {
            // live_in and live_out should contain no assigned(physical) registers
            for r in operand.registers() {
                self.propagate(cur_func, bb, r.id, liveness);
            }
        }
    }

    fn propagate(
        &self,
        cur_func: &MachineFunction,
        bb_id: MachineBasicBlockId,
        reg: RegisterId,
        liveness: &mut FxHashMap<MachineBasicBlockId, LivenessInfo>,
    ) {
        let bb = &cur_func.body.basic_blocks.arena[bb_id];

        {
            let liveness = liveness.get_mut(&bb_id).unwrap();

            if liveness.def.contains(&reg) {
                return;
            }

            if !liveness.add_live_in(reg) {
                // live_in already had the reg
                return;
            }
        }

        for pred_id in &bb.pred {
            if liveness.get_mut(pred_id).unwrap().add_live_out(reg) {
                // live_out didn't have the reg
                self.propagate(cur_func, *pred_id, reg, liveness);
            }
        }
    }

    pub fn construct_live_reg_matrix(&self, cur_func: &MachineFunction) -> LiveRegMatrix {
        let mut vreg2range: FxHashMap<VirtReg, LiveRange> = FxHashMap::default();
        let mut reg2range: PhysRegRange = PhysRegRange(FxHashMap::default());
        let mut id2pp: FxHashMap<MachineInstId, ProgramPoint> = FxHashMap::default();
        let mut virt_regs: FxHashMap<VirtReg, RegisterId> = FxHashMap::default();
        let mut program_points = ProgramPoints::new();

        let mut last_pp: ProgramPoint =
            program_points.new_program_point(ProgramPointBase::new(None, None, 0, 0));
        let mut bb_idx = 0;

        // TODO: Refine code

        // completely ignore callee saved registers

        for (id, _, iiter) in cur_func.body.mbb_iter() {
            let mut index = 0;
            let liveness = &cur_func.body.basic_blocks.liveness[&id];

            #[rustfmt::skip]
            macro_rules! cur_pp { () => {{
                last_pp
            }};}

            for livein in &liveness.live_in {
                if livein.is_virt_reg() {
                    vreg2range
                        .entry(livein.as_virt_reg())
                        .or_insert_with(|| LiveRange::new_empty())
                } else {
                    reg2range.get_or_create(livein.as_phys_reg().into())
                }
                .add_segment(LiveSegment::new(cur_pp!(), cur_pp!()))
            }

            index += IDX_STEP;
            last_pp = program_points
                .new_program_point(ProgramPointBase::new(None, None, bb_idx, index))
                .set_prev(Some(last_pp));

            for (inst_id, inst) in iiter {
                id2pp.insert(inst_id, cur_pp!());

                for reg in inst.collect_used_regs() {
                    if reg.id.is_phys_reg() && !is_callee_saved_reg(reg.id.as_phys_reg()) {
                        let phys_reg = reg.id.as_phys_reg();
                        if let Some(range) = reg2range.get_mut(phys_reg) {
                            range.segments.last_mut().unwrap().end = cur_pp!();
                        }
                    } else if reg.id.is_virt_reg() {
                        vreg2range
                            .get_mut(&reg.id.as_virt_reg())
                            .unwrap()
                            .segments
                            .last_mut()
                            .unwrap()
                            .end = cur_pp!();
                    }
                }

                // for &kill in &inst.kills {
                //     if kill.is_phys_reg() && !is_callee_saved_reg(kill.as_phys_reg()) {
                //         reg2range
                //             .get_or_create(kill.as_phys_reg())
                //             .add_segment(LiveSegment::new(cur_pp!(), cur_pp!()));
                //     } else if kill.is_virt_reg() {
                //         virt_regs.insert(kill.as_virt_reg(), *kill);
                //         vreg2range
                //             .entry(kill.as_virt_reg())
                //             .or_insert_with(|| LiveRange::new_empty())
                //             .add_segment(LiveSegment::new(cur_pp!(), cur_pp!()));
                //     }
                // }

                for def in inst.collect_defined_regs() {
                    if def.id.is_phys_reg() && !is_callee_saved_reg(def.id.as_phys_reg()) {
                        reg2range
                            .get_or_create(def.id.as_phys_reg())
                            .add_segment(LiveSegment::new(cur_pp!(), cur_pp!()));
                    } else if def.id.is_virt_reg() {
                        virt_regs.insert(def.id.as_virt_reg(), def.id);
                        vreg2range
                            .entry(def.id.as_virt_reg())
                            .or_insert_with(|| LiveRange::new_empty())
                            .add_segment(LiveSegment::new(cur_pp!(), cur_pp!()));
                    }
                }

                index += IDX_STEP;
                last_pp = program_points
                    .new_program_point(ProgramPointBase::new(None, None, bb_idx, index))
                    .set_prev(Some(last_pp));
            }

            for liveout in &liveness.live_out {
                if liveout.is_virt_reg() {
                    vreg2range.get_mut(&liveout.as_virt_reg())
                } else {
                    reg2range.get_mut(liveout.as_phys_reg())
                }
                .unwrap()
                .segments
                .last_mut()
                .unwrap()
                .end = cur_pp!();
            }

            bb_idx += 1;
            last_pp = program_points
                .new_program_point(ProgramPointBase::new(None, None, bb_idx, 0))
                .set_prev(Some(last_pp));
        }

        LiveRegMatrix::new(
            virt_regs,
            id2pp,
            VirtRegInterval(
                vreg2range
                    .into_iter()
                    .map(|(vreg, range)| (vreg, LiveInterval::new(vreg, range)))
                    .collect(),
            ),
            reg2range,
            program_points,
        )
    }
}

fn is_callee_saved_reg<T: TargetRegisterTrait>(r: T) -> bool {
    CALLEE_SAVED_REGS.with(|regs| regs.has(r.as_phys_reg()))
}
