use super::node::*;
use crate::codegen::arch::machine::register::{ty2rc, RegisterClassKind as RC};
use crate::codegen::common::{machine::register::RegistersInfo, types::MVType};
use crate::ir::types::Type;
use id_arena::Arena;
use rustc_hash::FxHashMap;
use std::ops::BitOr;

pub type GenFn = fn(&FxHashMap<&'static str, NodeId>, &mut MatchContext) -> NodeId;
pub type ReplacedNodeMap = FxHashMap<NodeId, NodeId>;
pub type NameMap = FxHashMap<&'static str, NodeId>;

pub struct MatchContext<'a> {
    pub arena: &'a mut Arena<Node>,
    pub regs: &'a RegistersInfo,
}

#[derive(Clone)]
pub enum Pat {
    IR(IRPat),
    MI,
    Operand(OperandPat),
    Compound(CompoundPat),
    Invalid,
}

#[derive(Clone)]
pub struct IRPat {
    pub name: &'static str,
    pub opcode: Option<IROpcode>,
    pub operands: Vec<Pat>,
    pub ty: Option<Type>,
    pub generate: Option<Box<GenFn>>,
}

#[derive(Clone)]
pub struct MIPat {}

#[derive(Clone)]
pub struct OperandPat {
    pub name: &'static str,
    pub kind: OperandKind,
    pub not: bool,
    pub generate: Option<Box<GenFn>>,
}

#[derive(Clone)]
pub struct CompoundPat {
    pub name: &'static str,
    pub pats: Vec<Pat>,
    pub generate: Option<Box<GenFn>>,
}

#[derive(Clone, Debug)]
pub enum OperandKind {
    Any,
    Imm(Immediate),
    Slot(Slot),
    Reg(Register),
    Block(Block),
    CC(Condition),
    Invalid,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Immediate {
    AnyInt8,
    AnyInt32,
    AnyInt64,
    AnyF64,
    AnyInt32PowerOf2,
    Int32(i32),
    Any,
    Null,
}

#[derive(Clone, Debug)]
pub enum Slot {
    Type(MVType),
    Any,
}

#[derive(Clone, Debug)]
pub enum Register {
    Class(RC),
    Any,
}

#[derive(Clone, Debug)]
pub enum Block {
    Any,
}

#[derive(Clone, Debug)]
pub enum Condition {
    Any,
}

pub const fn ir_node() -> IRPat {
    IRPat {
        name: "",
        opcode: None,
        operands: vec![],
        ty: None,
        generate: None,
    }
}

pub const fn ir(opcode: IROpcode) -> IRPat {
    IRPat {
        name: "",
        opcode: Some(opcode),
        operands: vec![],
        ty: None,
        generate: None,
    }
}

pub const fn mi_node() -> MIPat {
    MIPat {}
}

impl Pat {
    pub fn ty(mut self, ty: Type) -> Self {
        match &mut self {
            Self::IR(IRPat { ty: t, .. }) => *t = Some(ty),
            Self::MI | Self::Operand(_) | Self::Compound(_) | Self::Invalid => panic!(),
        }
        self
    }

    pub fn named(mut self, name: &'static str) -> Self {
        match &mut self {
            Self::IR(IRPat { name: n, .. }) => *n = name,
            Self::MI => panic!(),
            Self::Operand(OperandPat { name: n, .. }) => *n = name,
            Self::Compound(CompoundPat { name: n, .. }) => *n = name,
            Self::Invalid => panic!(),
        }
        self
    }

    pub fn generate(mut self, f: GenFn) -> Self {
        match &mut self {
            Self::IR(IRPat { generate, .. }) => *generate = Some(Box::new(f)),
            Self::MI => panic!(),
            Self::Operand(OperandPat { generate, .. }) => *generate = Some(Box::new(f)),
            Self::Compound(CompoundPat { generate, .. }) => *generate = Some(Box::new(f)),
            Self::Invalid => panic!(),
        }
        self
    }
}

impl IRPat {
    pub fn named(mut self, name: &'static str) -> Self {
        self.name = name;
        self
    }

    pub fn ty(mut self, ty: Type) -> Self {
        self.ty = Some(ty);
        self
    }

    pub fn opcode(mut self, opcode: IROpcode) -> Self {
        self.opcode = Some(opcode);
        self
    }

    pub fn args(mut self, operands: Vec<Pat>) -> Self {
        self.operands = operands;
        self
    }

    pub fn generate(mut self, f: GenFn) -> Self {
        self.generate = Some(Box::new(f));
        self
    }
}

impl OperandPat {
    pub fn named(mut self, name: &'static str) -> Self {
        self.name = name;
        self
    }

    pub fn any_imm(mut self) -> Self {
        self.kind = OperandKind::Imm(Immediate::Any);
        self
    }

    pub fn any_i32_imm(mut self) -> Self {
        self.kind = OperandKind::Imm(Immediate::AnyInt32);
        self
    }

    pub fn generate(mut self, f: GenFn) -> Self {
        self.generate = Some(Box::new(f));
        self
    }
}

impl CompoundPat {
    pub fn named(mut self, name: &'static str) -> Self {
        self.name = name;
        self
    }

    pub fn generate(mut self, f: GenFn) -> Self {
        self.generate = Some(Box::new(f));
        self
    }
}

pub const fn not() -> OperandPat {
    OperandPat {
        name: "",
        kind: OperandKind::Invalid,
        not: true,
        generate: None,
    }
}

pub const fn any() -> Pat {
    Pat::Operand(OperandPat {
        name: "",
        kind: OperandKind::Any,
        not: false,
        generate: None,
    })
}

pub const fn any_cc() -> OperandPat {
    OperandPat {
        name: "",
        kind: OperandKind::CC(Condition::Any),
        not: false,
        generate: None,
    }
}

pub const fn null_imm() -> OperandPat {
    OperandPat {
        name: "",
        kind: OperandKind::Imm(Immediate::Null),
        not: false,
        generate: None,
    }
}

pub const fn any_imm() -> OperandPat {
    OperandPat {
        name: "",
        kind: OperandKind::Imm(Immediate::Any),
        not: false,
        generate: None,
    }
}

pub const fn i32_imm(i: i32) -> OperandPat {
    OperandPat {
        name: "",
        kind: OperandKind::Imm(Immediate::Int32(i)),
        not: false,
        generate: None,
    }
}

pub const fn any_i8_imm() -> OperandPat {
    OperandPat {
        name: "",
        kind: OperandKind::Imm(Immediate::AnyInt8),
        not: false,
        generate: None,
    }
}

pub const fn any_imm8() -> Pat {
    Pat::Operand(OperandPat {
        name: "",
        kind: OperandKind::Imm(Immediate::AnyInt8),
        not: false,
        generate: None,
    })
}

pub const fn any_i32_imm() -> OperandPat {
    OperandPat {
        name: "",
        kind: OperandKind::Imm(Immediate::AnyInt32),
        not: false,
        generate: None,
    }
}

pub const fn any_imm32() -> Pat {
    Pat::Operand(OperandPat {
        name: "",
        kind: OperandKind::Imm(Immediate::AnyInt32),
        not: false,
        generate: None,
    })
}

pub const fn any_imm32_power_of_2() -> Pat {
    Pat::Operand(OperandPat {
        name: "",
        kind: OperandKind::Imm(Immediate::AnyInt32PowerOf2),
        not: false,
        generate: None,
    })
}

pub const fn any_i32_imm_power_of_2() -> Pat {
    Pat::Operand(OperandPat {
        name: "",
        kind: OperandKind::Imm(Immediate::AnyInt32PowerOf2),
        not: false,
        generate: None,
    })
}

pub const fn any_i64_imm() -> OperandPat {
    OperandPat {
        name: "",
        kind: OperandKind::Imm(Immediate::AnyInt64),
        not: false,
        generate: None,
    }
}

pub const fn any_imm_f64() -> Pat {
    Pat::Operand(OperandPat {
        name: "",
        kind: OperandKind::Imm(Immediate::AnyF64),
        not: false,
        generate: None,
    })
}

pub const fn any_f64_imm() -> OperandPat {
    OperandPat {
        name: "",
        kind: OperandKind::Imm(Immediate::AnyF64),
        not: false,
        generate: None,
    }
}

pub const fn any_slot() -> Pat {
    Pat::Operand(OperandPat {
        name: "",
        kind: OperandKind::Slot(Slot::Any),
        not: false,
        generate: None,
    })
}

pub const fn slot(ty: MVType) -> OperandPat {
    OperandPat {
        name: "",
        kind: OperandKind::Slot(Slot::Type(ty)),
        not: false,
        generate: None,
    }
}

pub const fn reg_class(rc: RC) -> OperandPat {
    OperandPat {
        name: "",
        kind: OperandKind::Reg(Register::Class(rc)),
        not: false,
        generate: None,
    }
}

pub const fn reg_(rc: RC) -> Pat {
    Pat::Operand(OperandPat {
        name: "",
        kind: OperandKind::Reg(Register::Class(rc)),
        not: false,
        generate: None,
    })
}

pub const fn any_reg() -> OperandPat {
    OperandPat {
        name: "",
        kind: OperandKind::Reg(Register::Any),
        not: false,
        generate: None,
    }
}

pub const fn any_block() -> OperandPat {
    OperandPat {
        name: "",
        kind: OperandKind::Block(Block::Any),
        not: false,
        generate: None,
    }
}

pub fn load(arg: Pat) -> Pat {
    ir(IROpcode::Load).args(vec![arg]).into()
}

pub fn store(src: Pat, dst: Pat) -> Pat {
    ir(IROpcode::Store).args(vec![src, dst]).into()
}

pub fn sext(arg: Pat) -> Pat {
    ir(IROpcode::Sext).args(vec![arg]).into()
}

pub fn add(lhs: Pat, rhs: Pat) -> Pat {
    ir(IROpcode::Add).args(vec![lhs, rhs]).into()
}

pub fn mul(lhs: Pat, rhs: Pat) -> Pat {
    ir(IROpcode::Mul).args(vec![lhs, rhs]).into()
}

pub fn fiaddr(slot: Pat) -> Pat {
    ir(IROpcode::FIAddr).args(vec![slot]).into()
}

pub fn gbladdr(g: Pat) -> Pat {
    ir(IROpcode::GlobalAddr).args(vec![g]).into()
}

pub fn bitcast(arg: Pat) -> Pat {
    ir(IROpcode::Bitcast).args(vec![arg]).into()
}

impl Into<Pat> for IRPat {
    fn into(self) -> Pat {
        Pat::IR(self)
    }
}

impl Into<Pat> for OperandPat {
    fn into(self) -> Pat {
        Pat::Operand(self)
    }
}

impl Into<CompoundPat> for OperandPat {
    fn into(self) -> CompoundPat {
        CompoundPat {
            name: "",
            pats: vec![self.into()],
            generate: None,
        }
    }
}

impl Into<CompoundPat> for IRPat {
    fn into(self) -> CompoundPat {
        CompoundPat {
            name: "",
            pats: vec![self.into()],
            generate: None,
        }
    }
}

impl Into<Pat> for CompoundPat {
    fn into(self) -> Pat {
        Pat::Compound(self)
    }
}

impl Into<Pat> for RC {
    fn into(self) -> Pat {
        Pat::Operand(OperandPat {
            name: "",
            kind: OperandKind::Reg(Register::Class(self)),
            not: false,
            generate: None,
        })
    }
}

impl BitOr for IRPat {
    type Output = CompoundPat;

    fn bitor(self, rhs: Self) -> Self::Output {
        CompoundPat {
            name: "",
            pats: vec![self.into(), rhs.into()],
            generate: None,
        }
    }
}

impl BitOr for OperandPat {
    type Output = CompoundPat;

    fn bitor(self, rhs: Self) -> Self::Output {
        CompoundPat {
            name: "",
            pats: vec![self.into(), rhs.into()],
            generate: None,
        }
    }
}

impl BitOr for CompoundPat {
    type Output = CompoundPat;

    fn bitor(self, rhs: Self) -> Self::Output {
        CompoundPat {
            name: "",
            pats: vec![self.pats, rhs.pats]
                .into_iter()
                .flatten()
                .collect::<Vec<Pat>>(),
            generate: None,
        }
    }
}

impl BitOr for Pat {
    type Output = Pat;

    fn bitor(self, rhs: Self) -> Self::Output {
        match self {
            Pat::Compound(mut c) => {
                c.pats.push(rhs);
                Pat::Compound(c)
            }
            Pat::IR(_) | Pat::Operand(_) => {
                let pats = vec![self, rhs];
                Pat::Compound(CompoundPat {
                    name: "",
                    pats,
                    generate: None,
                })
            }
            Pat::MI => todo!(),
            _ => panic!(),
        }
    }
}

// impl BitOr for IRPat {}

// if node.operand[0].is_operation()
//     && node.operand[0].kind == NodeKind::IR(IRNodeKind::Add)
//     && !node.operand[0].operand[0].is_constant()
//     && node.operand[0].operand[1].is_constant()
//     && node.operand[1].is_constant()
// {

impl Pat {
    pub fn depth(&self) -> usize {
        match self {
            Self::IR(ir) => ir.depth(),
            Self::MI => 0,
            Self::Operand(op) => op.depth(),
            Self::Compound(c) => c.depth(),
            Self::Invalid => panic!(),
        }
    }
}

impl IRPat {
    pub fn depth(&self) -> usize {
        let mut max = 0;
        for pat in &self.operands {
            let depth = pat.depth();
            if max < depth {
                max = depth
            }
        }
        max + 1
    }
}

impl OperandPat {
    pub fn depth(&self) -> usize {
        1
    }
}

impl CompoundPat {
    pub fn depth(&self) -> usize {
        let mut max = 0;
        for pat in &self.pats {
            let depth = pat.depth();
            if max < depth {
                max = depth
            }
        }
        max + 1
    }
}

pub fn reorder_patterns(pats: Vec<Pat>) -> Vec<Pat> {
    let mut pats_with_depth: Vec<(usize, Pat)> =
        pats.into_iter().map(|pat| (pat.depth(), pat)).collect();
    pats_with_depth.sort_by(|x, y| y.0.cmp(&x.0));
    pats_with_depth.into_iter().map(|(_, pat)| pat).collect()
}

#[test]
fn xxx() {
    let mut arena: Arena<Node> = Arena::new();
    let regs = RegistersInfo::new();

    // let reg = arena.alloc(Node::Operand(OperandNode::Reg(regs.new_virt_reg(RC::GR32))));
    let imm1 = arena.alloc(Node::Operand(OperandNode::Imm(ImmediateKind::Int32(2))));
    let imm2 = arena.alloc(Node::Operand(OperandNode::Imm(ImmediateKind::Int32(5))));
    let node = arena.alloc(
        IRNode::new(IROpcode::Sub)
            .args(vec![imm1, imm2])
            .ty(Type::i32)
            .into(),
    );
    // let add1 = arena.alloc(Node::IR(IRNode {
    //     opcode: IROpcode::Add,
    //     args: vec![reg, imm1],
    //     ty: Type::i32,
    //     mvty: MVType::i32,
    //     next: None,
    //     chain: None,
    // }));
    // let add2 = arena.alloc(Node::IR(IRNode {
    //     opcode: IROpcode::Add,
    //     args: vec![add1, imm2],
    //     ty: Type::i32,
    //     mvty: MVType::i32,
    //     next: None,
    //     chain: None,
    // }));
    // let node = add2;

    // ((X + C) + (3|5)) => (X + (C+3|C+5))
    let pat: Pat = ir(IROpcode::Add)
        .args(vec![
            ir(IROpcode::Add)
                .args(vec![
                    reg_class(RC::GR32).named("c").into(),
                    // not().any_i32_imm().named("c").into(),
                    any_i32_imm().named("d").into(),
                ])
                .into(),
            (i32_imm(3) | i32_imm(5)).named("e").into(),
        ])
        .generate(|m, c| {
            let lhs = m["c"];
            let rhs = c.arena[m["d"]].as_i32() + c.arena[m["e"]].as_i32();
            let rhs = c.arena.alloc(rhs.into());
            c.arena.alloc(
                IRNode::new(IROpcode::Add)
                    .args(vec![lhs, rhs])
                    .ty(Type::i32)
                    .into(),
            )
        })
        .into();

    let p: Pat = (ir(IROpcode::Add)
        .args(vec![any_i32_imm().named("i").into(), any_i32_imm().into()])
        | ir(IROpcode::Sub).args(vec![any_i32_imm().named("i").into(), any_i32_imm().into()]))
    .named("x")
    .generate(|m, c| {
        let id = m["x"];
        match c.arena[id].as_ir().opcode {
            IROpcode::Add => m["i"],
            IROpcode::Sub => m["i"],
            _ => unreachable!(),
        }
    })
    .into();

    let pats = vec![
        pat,
        p,
        i32_imm(7)
            .named("E")
            .generate(|_, c| c.arena.alloc(OperandNode::i32(11).into()))
            .into(),
    ];

    let _new_node = inst_select(
        &mut ReplacedNodeMap::default(),
        &mut MatchContext {
            arena: &mut arena,
            regs: &regs,
        },
        node,
        &pats,
    );

    // println!("{:#?}", arena);
    // println!("{:?}: {:?}", new_node, arena[new_node]);

    // let mut map = NameMap::default();
    // if let Some(gen) = try_match(&arena, node, &pat, &mut map) {
    //     for (_, &id) in &map {
    //         try_match(&arena, id, &pat, &mut NameMap::default());
    //     }
    //     let _id = gen(&map, &mut arena);
    //     return;
    // }

    // panic!()
}

pub fn inst_select(
    replaced: &mut ReplacedNodeMap,
    ctx: &mut MatchContext,
    id: NodeId,
    pats: &[Pat],
) -> NodeId {
    if let Some(replaced) = replaced.get(&id) {
        return *replaced;
    }

    // println!("{:?} = {:?}", id, ctx.arena[id]);

    let mut map = NameMap::default();
    for pat in pats {
        if let Some(gen) = try_match(ctx, id, &pat, &mut map) {
            for (_, named_id) in &mut map {
                // If current node (`id`) is named, `inst_select` infinitely recurses.
                // To avoid it, do not `inst_select` current node.
                if named_id == &id {
                    continue;
                }
                *named_id = inst_select(replaced, ctx, *named_id, pats);
            }
            let new_id = gen(&map, ctx);
            if id != new_id {
                replaced.insert(id, new_id);
            }
            return inst_select(replaced, ctx, new_id, pats);
        }
    }

    operand_select(replaced, ctx, id, pats);

    id
}

fn operand_select(
    replaced: &mut ReplacedNodeMap,
    ctx: &mut MatchContext,
    inst_id: NodeId,
    pats: &[Pat],
) {
    let args_id: Vec<NodeId> = ctx.arena[inst_id].args().into_iter().map(|x| *x).collect();
    let mut replaced_args = ReplacedNodeMap::default();
    for id in args_id {
        let new_id = inst_select(replaced, ctx, id, pats);
        replaced_args.insert(id, new_id);
    }
    // Actually replace args
    for id in ctx.arena[inst_id].args_mut() {
        *id = replaced_args[id];
    }
}

fn try_match(ctx: &mut MatchContext, id: NodeId, pat: &Pat, m: &mut NameMap) -> Option<Box<GenFn>> {
    if let Some(f) = matches(ctx, id, pat, m) {
        return f;
    }
    return None;
}

fn matches(
    ctx: &MatchContext,
    id: NodeId,
    pat: &Pat,
    m: &mut NameMap,
) -> Option<Option<Box<GenFn>>> {
    match pat {
        Pat::IR(pat) => {
            let n = match &ctx.arena[id] {
                Node::IR(ir) => ir,
                _ => return None,
            };
            let same_opcode = Some(n.opcode) == pat.opcode;
            if !same_opcode {
                return None;
            }
            let same_operands = pat
                .operands
                .iter()
                .zip(n.args.iter())
                .all(|(pat, &id)| matches(ctx, id, pat, m).is_some());
            if !same_operands {
                return None;
            }
            let same_ty = pat.ty.map_or(true, |ty| n.mvty == ty.into());
            if same_opcode && same_operands && same_ty {
                if !pat.name.is_empty() {
                    m.insert(pat.name, id);
                }
                Some(pat.generate.clone())
            } else {
                None
            }
        }
        Pat::MI => todo!(), // Some(None),
        Pat::Operand(op) => {
            let matches_ = match &ctx.arena[id] {
                Node::Operand(n) => {
                    let matches_ = match &op.kind {
                        OperandKind::Any => true,
                        OperandKind::Imm(Immediate::Null) => {
                            matches!(n, &OperandNode::Imm(i) if i.is_null())
                        }
                        OperandKind::Imm(Immediate::AnyInt8) => {
                            matches!(n, &OperandNode::Imm(ImmediateKind::Int8(_)))
                        }
                        OperandKind::Imm(Immediate::AnyInt32) => {
                            matches!(n, &OperandNode::Imm(ImmediateKind::Int32(_)))
                        }
                        OperandKind::Imm(Immediate::AnyInt64) => {
                            matches!(n, &OperandNode::Imm(ImmediateKind::Int64(_)))
                        }
                        OperandKind::Imm(Immediate::AnyF64) => {
                            matches!(n, &OperandNode::Imm(ImmediateKind::F64(_)))
                        }
                        OperandKind::Imm(Immediate::Int32(i)) => {
                            matches!(n, &OperandNode::Imm(ImmediateKind::Int32(x)) if x == *i)
                        }
                        OperandKind::Imm(Immediate::AnyInt32PowerOf2) => {
                            matches!(n, &OperandNode::Imm(ImmediateKind::Int32(x)) if (x as usize).is_power_of_two())
                        }
                        OperandKind::Imm(Immediate::Any) => matches!(n, &OperandNode::Imm(_)),
                        OperandKind::Reg(Register::Class(reg_class)) => {
                            matches!(n, OperandNode::Reg(id)
                                                if ctx.regs.arena_ref()[*id].reg_class == *reg_class)
                        }
                        OperandKind::Reg(Register::Any) => matches!(n, OperandNode::Reg(_)),
                        OperandKind::Slot(Slot::Any) => matches!(n, &OperandNode::Slot(_)),
                        OperandKind::Slot(Slot::Type(ty)) => {
                            matches!(n, &OperandNode::Slot(slot) if ty == &slot.ty.into())
                        }
                        OperandKind::Block(_) => matches!(n, &OperandNode::Block(_)),
                        OperandKind::CC(Condition::Any) => matches!(n, &OperandNode::CC(_)),
                        OperandKind::Invalid => panic!(),
                    };
                    if if op.not { !matches_ } else { matches_ } {
                        Some(op.generate.clone())
                    } else {
                        None
                    }
                }
                Node::IR(IRNode { ty, .. }) => match &op.kind {
                    OperandKind::Any => Some(op.generate.clone()),
                    OperandKind::Reg(Register::Class(reg_class))
                        if ty2rc(ty).unwrap() == *reg_class =>
                    {
                        Some(op.generate.clone())
                    }
                    OperandKind::Reg(Register::Any) if !matches!(ty, Type::Void) => {
                        Some(op.generate.clone())
                    }
                    OperandKind::Imm(_)
                    | OperandKind::Reg(_)
                    | OperandKind::Block(_)
                    | OperandKind::CC(_)
                    | OperandKind::Slot(_) => None,
                    OperandKind::Invalid => panic!(),
                },
                Node::MI(MINode { reg_class: rc, .. }) => match &op.kind {
                    OperandKind::Any => Some(op.generate.clone()),
                    OperandKind::Reg(Register::Class(reg_class)) if rc == &Some(*reg_class) => {
                        Some(op.generate.clone())
                    }
                    OperandKind::Reg(Register::Any) if rc.is_some() => Some(op.generate.clone()),
                    OperandKind::Imm(_)
                    | OperandKind::Reg(_)
                    | OperandKind::Block(_)
                    | OperandKind::CC(_)
                    | OperandKind::Slot(_) => None,
                    OperandKind::Invalid => panic!(),
                },
                Node::None => None,
            };
            if matches_.is_some() && !op.name.is_empty() {
                m.insert(op.name, id);
            }
            matches_
        }
        Pat::Compound(pat) => {
            let matches_ = pat.pats.iter().find_map(|pat| matches(ctx, id, pat, m));
            if matches_.is_some() && !pat.name.is_empty() {
                m.insert(pat.name, id);
            }
            if matches!(matches_, Some(None)) {
                Some(pat.generate.clone())
            } else {
                matches_
            }
        }
        Pat::Invalid => panic!(),
    }
}
