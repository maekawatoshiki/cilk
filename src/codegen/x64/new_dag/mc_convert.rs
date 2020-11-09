// use super::{node, node::*};
use crate::codegen::arch::frame_object::FrameIndexInfo;
use crate::codegen::arch::machine::abi::SystemV;
use crate::codegen::arch::machine::inst::*;
use crate::codegen::arch::machine::register::*;
use crate::codegen::common::machine::calling_conv::{ArgumentRegisterOrder, CallingConv};
use crate::codegen::common::machine::inst_def::DefOrUseReg;
use crate::codegen::common::machine::{inst, inst::*, register::*};
pub use crate::codegen::common::new_dag::mc_convert::ScheduleContext;
use crate::codegen::common::new_dag::node::{
    IRNode, IROpcode, ImmediateKind, Node, NodeId, OperandNode,
};
use crate::ir::types::*;
use crate::util::allocator::*;

impl<'a> ScheduleContext<'a> {
    pub fn convert_node(&mut self, id: NodeId) -> MachineInstId {
        if let Some(inst_id) = self.node2inst.get(&id) {
            return *inst_id;
        }

        let inst_id = match &self.func.node_arena[id] {
            Node::IR(IRNode {
                opcode: IROpcode::Ret,
                args,
                ..
            }) => self.convert_ret(args[0]),
            _ => todo!(),
        };
        inst_id
    }

    fn convert_ret(&mut self, arg: NodeId) -> MachineInstId {
        let arg = self.normal_arg(arg);

        match &arg {
            MachineOperand::Constant(MachineConstant::Int32(_)) => {
                let mov = MachineInst::new_simple(MachineOpcode::MOVri32, vec![arg], self.block_id)
                    .with_def(vec![RegisterOperand::new(
                        self.func.regs.get_phys_reg(GR32::EAX),
                    )]);
                self.append_inst(mov);
            }
            _ => todo!(),
        }

        self.append_inst(MachineInst::new_simple(
            MachineOpcode::RET,
            vec![],
            self.block_id,
        ))
    }

    pub fn normal_arg(&mut self, arg: NodeId) -> MachineOperand {
        match &self.func.node_arena[arg] {
            Node::Operand(OperandNode::Imm(ImmediateKind::Int32(i))) => {
                MachineOperand::Constant(MachineConstant::Int32(*i))
            }
            _ => todo!(),
        }
    }
}

// impl<'a> ScheduleByBlock<'a> {
//     pub fn convert_node_to_inst(&mut self, node: Raw<DAGNode>) -> MachineInstId {
//         if let Some(inst_id) = self.node2inst.get(&node) {
//             return *inst_id;
//         }
//
//         #[rustfmt::skip]
//         macro_rules! cond_kind {($id:expr)=>{ $id.as_cond_kind() };}
//
//         let inst_id = match &node.kind {
//             NodeKind::MI(_) => {
//                 fn reg(inst: &MachineInst, x: &DefOrUseReg) -> RegisterOperand {
//                     match x {
//                         DefOrUseReg::Def(i) => inst.def[*i],
//                         DefOrUseReg::Use(i) => *inst.operand[*i].as_register(),
//                     }
//                 }
//                 let mi = node.kind.as_mi();
//                 let inst_def = mi.inst_def().unwrap();
//                 let operands = node
//                     .operand
//                     .iter()
//                     .map(|op| self.normal_operand(*op))
//                     .collect();
//                 let mut inst = MachineInst::new(
//                     &self.cur_func.regs_info,
//                     mi,
//                     operands,
//                     ty2rc(&node.ty),
//                     self.cur_bb,
//                 );
//                 for (def_, use_) in &inst_def.tie {
//                     inst.tie_regs(reg(&inst, def_), reg(&inst, use_));
//                 }
//                 self.append_inst(inst)
//             }
//             NodeKind::IR(IRNodeKind::CopyToReg) => {
//                 let val = self.normal_operand(node.operand[1]);
//                 let dst = match &node.operand[0].kind {
//                     NodeKind::Operand(OperandNodeKind::Register(r)) => RegisterOperand::new(*r),
//                     _ => unreachable!(),
//                 };
//                 self.append_inst(MachineInst::new_with_def_reg(
//                     MachineOpcode::Copy,
//                     vec![val],
//                     vec![dst],
//                     self.cur_bb,
//                 ))
//             }
//             NodeKind::IR(IRNodeKind::Call) => self.convert_call_dag(&*node),
//             NodeKind::IR(IRNodeKind::Phi) => {
//                 let mut operands = vec![];
//                 let mut i = 0;
//                 while i < node.operand.len() {
//                     operands.push(self.normal_operand(node.operand[i]));
//                     operands.push(MachineOperand::Branch(
//                         self.get_machine_bb(node.operand[i + 1].as_basic_block()),
//                     ));
//                     i += 2;
//                 }
//                 let phi_inst = MachineInst::new(
//                     &self.cur_func.regs_info,
//                     MachineOpcode::Phi,
//                     operands,
//                     ty2rc(&node.ty),
//                     self.cur_bb,
//                 );
//                 self.append_inst(phi_inst)
//             }
//             NodeKind::IR(IRNodeKind::Div) => {
//                 let regs = match node.ty {
//                     Type::i8 => to_phys!(GR32::EAX, GR32::EDX),
//                     Type::i32 => to_phys!(GR32::EAX, GR32::EDX),
//                     // Type::i64 => to_phys!(GR64::RAX, GR64::RDX),
//                     _ => unimplemented!(),
//                 };
//                 let eax = RegisterOperand::new(self.cur_func.regs_info.get_phys_reg(regs[0]));
//                 let edx = RegisterOperand::new(self.cur_func.regs_info.get_phys_reg(regs[1]));
//
//                 let mut op1 = self.normal_operand(node.operand[0]);
//                 let mut op2 = self.normal_operand(node.operand[1]);
//
//                 // TODO: special case
//                 if node.ty == Type::i8 {
//                     if let MachineOperand::Register(r) = &mut op1 {
//                         *r = r.sub_super(Some(RegisterClassKind::GR32))
//                     }
//                     if let MachineOperand::Register(r) = &mut op2 {
//                         *r = r.sub_super(Some(RegisterClassKind::GR32))
//                     }
//                 }
//
//                 self.append_inst(
//                     MachineInst::new_simple(
//                         mov_r_x(regs[0].reg_class(), &op1).unwrap(),
//                         vec![op1],
//                         self.cur_bb,
//                     )
//                     .with_def(vec![eax]),
//                 );
//
//                 self.append_inst(
//                     MachineInst::new_simple(MachineOpcode::CDQ, vec![], self.cur_bb)
//                         .with_imp_defs(vec![eax, edx])
//                         .with_imp_use(eax),
//                 );
//
//                 // assert_eq!(op2.get_type(&self.cur_func.regs_info), Some(Type::i32));
//                 let inst1 = MachineInst::new(
//                     &self.cur_func.regs_info,
//                     mov_r_x(regs[0].reg_class(), &op2).unwrap(),
//                     vec![op2],
//                     Some(regs[0].reg_class()), // TODO: support other types
//                     self.cur_bb,
//                 );
//                 let op2 = MachineOperand::Register(inst1.def[0]);
//                 self.append_inst(inst1);
//
//                 self.append_inst(
//                     MachineInst::new_simple(MachineOpcode::IDIV, vec![op2], self.cur_bb)
//                         .with_imp_defs(vec![eax, edx])
//                         .with_imp_uses(vec![eax, edx]),
//                 );
//
//                 let copy_inst = MachineInst::new(
//                     &self.cur_func.regs_info,
//                     MachineOpcode::Copy,
//                     vec![MachineOperand::Register(eax)],
//                     Some(regs[0].reg_class()), // TODO
//                     self.cur_bb,
//                 );
//                 self.append_inst(copy_inst)
//             }
//             NodeKind::IR(IRNodeKind::Rem) => {
//                 let eax = RegisterOperand::new(self.cur_func.regs_info.get_phys_reg(GR32::EAX));
//                 let edx = RegisterOperand::new(self.cur_func.regs_info.get_phys_reg(GR32::EDX));
//
//                 let op1 = self.normal_operand(node.operand[0]);
//                 let op2 = self.normal_operand(node.operand[1]);
//
//                 self.append_inst(
//                     MachineInst::new_simple(mov_n_rx(32, &op1).unwrap(), vec![op1], self.cur_bb)
//                         .with_def(vec![eax]),
//                 );
//
//                 self.append_inst(
//                     MachineInst::new_simple(MachineOpcode::CDQ, vec![], self.cur_bb)
//                         .with_imp_defs(vec![eax, edx])
//                         .with_imp_use(eax),
//                 );
//
//                 assert_eq!(op2.get_type(&self.cur_func.regs_info), Some(Type::i32));
//                 let inst1 = MachineInst::new(
//                     &self.cur_func.regs_info,
//                     mov_n_rx(32, &op2).unwrap(),
//                     vec![op2],
//                     Some(RegisterClassKind::GR32), // TODO: support other types
//                     self.cur_bb,
//                 );
//                 let op2 = MachineOperand::Register(inst1.def[0]);
//                 self.append_inst(inst1);
//
//                 self.append_inst(
//                     MachineInst::new_simple(MachineOpcode::IDIV, vec![op2], self.cur_bb)
//                         .with_imp_defs(vec![eax, edx])
//                         .with_imp_uses(vec![eax, edx]),
//                 );
//
//                 self.append_inst(MachineInst::new(
//                     &self.cur_func.regs_info,
//                     MachineOpcode::Copy,
//                     vec![MachineOperand::Register(edx)],
//                     Some(RegisterClassKind::GR32), // TODO
//                     self.cur_bb,
//                 ))
//             }
//             NodeKind::IR(IRNodeKind::Setcc) => {
//                 let new_op1 = self.normal_operand(node.operand[1]);
//                 let new_op2 = self.normal_operand(node.operand[2]);
//                 let inst = MachineInst::new(
//                     &self.cur_func.regs_info,
//                     match cond_kind!(node.operand[0]) {
//                         CondKind::Eq => MachineOpcode::Seteq,
//                         CondKind::Le => MachineOpcode::Setle,
//                         CondKind::Lt => MachineOpcode::Setlt,
//                         _ => unimplemented!(),
//                     },
//                     vec![new_op1, new_op2],
//                     ty2rc(&node.ty),
//                     self.cur_bb,
//                 );
//                 self.append_inst(inst)
//             }
//             NodeKind::IR(IRNodeKind::Brcc) => {
//                 let op0 = self.normal_operand(node.operand[1]);
//                 let op1 = self.normal_operand(node.operand[2]);
//
//                 self.append_inst(MachineInst::new_simple(
//                     if op0.is_register() && op1.is_constant() {
//                         MachineOpcode::CMPri
//                     } else if op0.is_register() && op1.is_register() {
//                         MachineOpcode::CMPrr
//                     } else {
//                         unreachable!()
//                     },
//                     vec![op0, op1],
//                     self.cur_bb,
//                 ));
//
//                 self.append_inst(MachineInst::new_simple(
//                     match cond_kind!(node.operand[0]) {
//                         CondKind::Eq => MachineOpcode::JE,
//                         CondKind::Ne => MachineOpcode::JNE,
//                         CondKind::Le => MachineOpcode::JLE,
//                         CondKind::Lt => MachineOpcode::JL,
//                         CondKind::Ge => MachineOpcode::JGE,
//                         CondKind::Gt => MachineOpcode::JG,
//                         _ => unreachable!(),
//                     },
//                     vec![MachineOperand::Branch(
//                         self.get_machine_bb(node.operand[3].as_basic_block()),
//                     )],
//                     self.cur_bb,
//                 ))
//             }
//             NodeKind::IR(IRNodeKind::FPBrcc) => {
//                 let op0 = self.normal_operand(node.operand[1]);
//                 let op1 = self.normal_operand(node.operand[2]);
//
//                 self.append_inst(MachineInst::new_simple(
//                     MachineOpcode::UCOMISDrr,
//                     vec![op0, op1],
//                     self.cur_bb,
//                 ));
//
//                 self.append_inst(MachineInst::new_simple(
//                     match cond_kind!(node.operand[0]) {
//                         CondKind::UEq => MachineOpcode::JE,
//                         CondKind::UNe => MachineOpcode::JNE,
//                         CondKind::ULe => MachineOpcode::JBE,
//                         CondKind::ULt => MachineOpcode::JB,
//                         CondKind::UGe => MachineOpcode::JAE,
//                         CondKind::UGt => MachineOpcode::JA,
//                         _ => unreachable!(),
//                     },
//                     vec![MachineOperand::Branch(
//                         self.get_machine_bb(node.operand[3].as_basic_block()),
//                     )],
//                     self.cur_bb,
//                 ))
//             }
//             NodeKind::IR(IRNodeKind::Ret) => self.convert_ret(&*node),
//             NodeKind::IR(IRNodeKind::CopyToLiveOut) => self.convert_node_to_inst(node.operand[0]),
//             e => panic!("{:?}, {:?}", e, node.ty),
//         };
//
//         self.node2inst.insert(node, inst_id);
//
//         inst_id
//     }
//
//     pub fn convert_ret(&mut self, node: &DAGNode) -> MachineInstId {
//         let val = self.normal_operand(node.operand[0]);
//         println!("{:?}", val);
//
//         if let Some(ty) = val.get_type(&self.cur_func.regs_info) {
//             let ret_reg = ty2rc(&ty).unwrap().return_value_register();
//             let set_ret_val = MachineInst::new_simple(
//                 mov_rx(self.types, &self.cur_func.regs_info, &val).unwrap(),
//                 vec![val],
//                 self.cur_bb,
//             )
//             .with_def(vec![RegisterOperand::new(
//                 self.cur_func.regs_info.get_phys_reg(ret_reg),
//             )]);
//             self.append_inst(set_ret_val);
//         }
//
//         self.append_inst(MachineInst::new_simple(
//             MachineOpcode::RET,
//             vec![],
//             self.cur_bb,
//         ))
//     }
//
//     fn move2reg(&self, r: RegisterId, src: MachineOperand) -> MachineInst {
//         let opcode = mov_rx(self.types, &self.cur_func.regs_info, &src).unwrap();
//         MachineInst::new_simple(opcode, vec![src], self.cur_bb)
//             .with_def(vec![RegisterOperand::new(r)])
//     }
//
//     fn convert_call_dag(&mut self, node: &DAGNode) -> MachineInstId {
//         let mut arg_regs = vec![RegisterOperand::new(
//             self.cur_func.regs_info.get_phys_reg(GR64::RSP),
//         )]; // call uses RSP
//         let mut off = 0i32;
//
//         // println!("T {:?}", self.types.to_string(node.operand[0].ty));
//         let f_ty = node.operand[0].ty;
//
//         let mut args = vec![];
//         for (i, operand) in node.operand[1..].iter().enumerate() {
//             let byval = self
//                 .types
//                 .base
//                 .borrow()
//                 .as_function_ty(f_ty)
//                 .unwrap()
//                 .params_attr
//                 .get(&i)
//                 .map_or(false, |attr| attr.byval);
//             if byval {
//                 args.push(MachineOperand::None);
//             } else {
//                 args.push(self.normal_operand(*operand));
//             }
//         }
//
//         let abi = SystemV::new();
//         let mut arg_regs_order = ArgumentRegisterOrder::new(&abi);
//
//         for (i, arg) in args.into_iter().enumerate() {
//             let (ty, byval) = {
//                 let base = self.types.base.borrow();
//                 let f = &base.as_function_ty(f_ty).unwrap();
//                 (
//                     *f.params_ty.get(i).unwrap(),
//                     f.params_attr.get(&i).map_or(false, |attr| attr.byval),
//                 )
//             };
//
//             if byval {
//                 // TODO
//                 let lea = &node.operand[1 + i];
//                 let mem = lea.operand[0];
//                 let fi = match self.normal_operand(mem) {
//                     MachineOperand::Mem(MachineMemOperand::BaseFi(_, fi)) => fi,
//                     _ => panic!(),
//                 };
//                 arg_regs.append(&mut self.pass_struct_byval(&mut arg_regs_order, &mut off, ty, fi));
//                 continue;
//             }
//
//             if !matches!(
//                 ty,
//                 Type::i8 | Type::i32 | Type::i64 | Type::f64 | Type::Pointer(_) | Type::Array(_)
//             ) {
//                 unimplemented!()
//             };
//
//             let reg_class = ty2rc(&ty).unwrap();
//             let inst = match arg_regs_order.next(reg_class) {
//                 Some(arg_reg) => {
//                     let r = self.cur_func.regs_info.get_phys_reg(arg_reg);
//                     arg_regs.push(RegisterOperand::new(r));
//                     self.move2reg(r, arg)
//                 }
//                 None => {
//                     // Put the exceeded value onto the stack
//                     let inst = MachineInst::new_simple(
//                         mov_mx(&self.cur_func.regs_info, &arg).unwrap(),
//                         vec![
//                             MachineOperand::Mem(MachineMemOperand::BaseOff(
//                                 RegisterOperand::new(
//                                     self.cur_func.regs_info.get_phys_reg(GR64::RSP),
//                                 ),
//                                 off,
//                             )),
//                             arg,
//                         ],
//                         self.cur_bb,
//                     );
//                     off += 8;
//                     inst
//                 }
//             };
//
//             self.append_inst(inst);
//         }
//
//         self.append_inst(
//             MachineInst::new_simple(
//                 MachineOpcode::AdjStackDown,
//                 vec![MachineOperand::imm_i32(off)],
//                 self.cur_bb,
//             )
//             .with_imp_def(RegisterOperand::new(
//                 self.cur_func.regs_info.get_phys_reg(GR64::RSP),
//             ))
//             .with_imp_use(RegisterOperand::new(
//                 self.cur_func.regs_info.get_phys_reg(GR64::RSP),
//             )),
//         );
//
//         let callee = self.normal_operand(node.operand[0]);
//         let ret_reg = self.cur_func.regs_info.get_phys_reg(
//             ty2rc(&node.ty)
//                 .unwrap_or(RegisterClassKind::GR32)
//                 .return_value_register(),
//         );
//         let call_inst = self.append_inst(
//             MachineInst::new_simple(MachineOpcode::CALL, vec![callee], self.cur_bb)
//                 .with_imp_uses(arg_regs)
//                 .with_imp_defs({
//                     let mut defs = vec![RegisterOperand::new(
//                         self.cur_func.regs_info.get_phys_reg(GR64::RSP),
//                     )];
//                     if node.ty != Type::Void {
//                         defs.push(RegisterOperand::new(ret_reg))
//                     }
//                     defs
//                 }),
//         );
//
//         self.append_inst(
//             MachineInst::new_simple(
//                 MachineOpcode::AdjStackUp,
//                 vec![MachineOperand::imm_i32(off)],
//                 self.cur_bb,
//             )
//             .with_imp_def(RegisterOperand::new(
//                 self.cur_func.regs_info.get_phys_reg(GR64::RSP),
//             ))
//             .with_imp_use(RegisterOperand::new(
//                 self.cur_func.regs_info.get_phys_reg(GR64::RSP),
//             )),
//         );
//
//         if node.ty == Type::Void {
//             return call_inst;
//         }
//
//         let reg_class = self.cur_func.regs_info.arena_ref()[ret_reg].reg_class;
//         let copy = MachineInst::new(
//             &self.cur_func.regs_info,
//             MachineOpcode::Copy,
//             vec![MachineOperand::Register(RegisterOperand::new(ret_reg))],
//             Some(reg_class),
//             self.cur_bb,
//         );
//         self.append_inst(copy)
//     }
//
//     fn pass_struct_byval<ABI>(
//         &mut self,
//         arg_regs_order: &mut ArgumentRegisterOrder<ABI>,
//         off: &mut i32,
//         ty: Type,
//         fi: FrameIndexInfo,
//     ) -> Vec<RegisterOperand>
//     where
//         ABI: CallingConv,
//     {
//         let mut arg_regs = vec![];
//         let struct_ty = self.cur_func.types.get_element_ty(ty, None).unwrap();
//         let base = &self.cur_func.types.base.borrow();
//         let struct_ty = base.as_struct_ty(struct_ty).unwrap();
//         let sz = struct_ty.size();
//         let mov8 = sz / 8;
//         let mov4 = (sz - 8 * mov8) / 4;
//         let rbp = RegisterOperand::new(self.cur_func.regs_info.get_phys_reg(GR64::RBP));
//         assert!((sz - 8 * mov8) % 4 == 0);
//         let regs_classes = SystemV::reg_classes_used_for_passing_byval(struct_ty);
//
//         if sz <= 16 && arg_regs_order.regs_available_for(&regs_classes) {
//             let mut off = 0;
//             for &rc in &regs_classes {
//                 let r = RegisterOperand::new(
//                     self.cur_func
//                         .regs_info
//                         .get_phys_reg(arg_regs_order.next(rc).unwrap()),
//                 );
//                 arg_regs.push(r);
//
//                 let mem = MachineOperand::Mem(if off == 0 {
//                     MachineMemOperand::BaseFi(rbp, fi.clone())
//                 } else {
//                     MachineMemOperand::BaseFiOff(rbp, fi.clone(), off as i32)
//                 });
//
//                 let mov = MachineInst::new_simple(
//                     match rc {
//                         RegisterClassKind::GR32 => MachineOpcode::MOVrm32,
//                         RegisterClassKind::GR64 => MachineOpcode::MOVrm64,
//                         RegisterClassKind::XMM => MachineOpcode::MOVSDrm,
//                         RegisterClassKind::GR8 => unimplemented!(),
//                     },
//                     vec![mem],
//                     self.cur_bb,
//                 )
//                 .with_def(vec![r]);
//                 self.append_inst(mov);
//                 off += match rc {
//                     RegisterClassKind::XMM => 8,
//                     _ => rc.size_in_byte(),
//                 };
//             }
//             return arg_regs;
//         }
//
//         let mut offset = 0;
//         for (c, s, rc, op) in vec![
//             (mov8, 8, RegisterClassKind::GR64, MachineOpcode::MOVrm64),
//             (mov4, 4, RegisterClassKind::GR32, MachineOpcode::MOVrm32),
//         ]
//         .into_iter()
//         {
//             for _ in 0..c {
//                 let r = RegisterOperand::new(self.cur_func.regs_info.new_virt_reg(rc));
//                 let mem = if offset == 0 {
//                     MachineOperand::Mem(MachineMemOperand::BaseFi(rbp, fi.clone()))
//                 } else {
//                     MachineOperand::Mem(MachineMemOperand::BaseFiOff(rbp, fi.clone(), offset))
//                 };
//                 let mov = MachineInst::new_simple(op, vec![mem], self.cur_bb).with_def(vec![r]);
//                 self.append_inst(mov);
//                 let mov = MachineInst::new_simple(
//                     MachineOpcode::MOVmr64,
//                     vec![
//                         MachineOperand::Mem(MachineMemOperand::BaseOff(
//                             RegisterOperand::new(self.cur_func.regs_info.get_phys_reg(GR64::RSP)),
//                             *off + offset as i32,
//                         )),
//                         MachineOperand::Register(r),
//                     ],
//                     self.cur_bb,
//                 );
//                 self.append_inst(mov);
//                 offset += s;
//             }
//         }
//         *off += sz as i32;
//
//         vec![]
//     }
//
//     pub fn normal_operand(&mut self, node: Raw<DAGNode>) -> MachineOperand {
//         match node.kind {
//             NodeKind::Operand(OperandNodeKind::Constant(c)) => match c {
//                 ConstantKind::Int8(i) => MachineOperand::Constant(MachineConstant::Int8(i)),
//                 ConstantKind::Int32(i) => MachineOperand::Constant(MachineConstant::Int32(i)),
//                 ConstantKind::Int64(i) => MachineOperand::Constant(MachineConstant::Int64(i)),
//                 ConstantKind::F64(f) => MachineOperand::Constant(MachineConstant::F64(f)),
//                 ConstantKind::Other(c) => {
//                     MachineOperand::Mem(MachineMemOperand::Address(inst::AddressKind::Constant(c)))
//                 }
//             },
//             NodeKind::Operand(OperandNodeKind::FrameIndex(ref kind)) => {
//                 MachineOperand::FrameIndex(kind.clone())
//             }
//             NodeKind::Operand(OperandNodeKind::Address(ref g)) => match g {
//                 node::AddressKind::FunctionName(n) => MachineOperand::Mem(
//                     MachineMemOperand::Address(inst::AddressKind::FunctionName(n.clone())),
//                 ),
//                 _ => unreachable!()
//                 // node::AddressKind::GlobalName(n) => MachineOperand::Mem(
//                 //     MachineMemOperand::Address(inst::AddressKind::GlobalName(n.clone())),
//                 // ),
//             },
//             NodeKind::Operand(OperandNodeKind::BasicBlock(id)) => {
//                 MachineOperand::Branch(self.get_machine_bb(id))
//             }
//             NodeKind::Operand(OperandNodeKind::Register(ref r)) => {
//                 MachineOperand::Register(RegisterOperand::new(*r))
//             }
//             NodeKind::Operand(OperandNodeKind::Mem(ref mem)) => match mem {
//                 MemNodeKind::Base => MachineOperand::Mem(MachineMemOperand::Base(
//                     *self.normal_operand(node.operand[0]).as_register(),
//                 )),
//                 MemNodeKind::BaseAlignOff => MachineOperand::Mem(MachineMemOperand::BaseAlignOff(
//                     *self.normal_operand(node.operand[0]).as_register(),
//                     self.normal_operand(node.operand[1]).as_constant().as_i32(),
//                     *self.normal_operand(node.operand[2]).as_register(),
//                 )),
//                 MemNodeKind::BaseFi => MachineOperand::Mem(MachineMemOperand::BaseFi(
//                     *self.normal_operand(node.operand[0]).as_register(),
//                     *self.normal_operand(node.operand[1]).as_frame_index(),
//                 )),
//                 MemNodeKind::BaseFiAlignOff => {
//                     MachineOperand::Mem(MachineMemOperand::BaseFiAlignOff(
//                         *self.normal_operand(node.operand[0]).as_register(),
//                         *self.normal_operand(node.operand[1]).as_frame_index(),
//                         self.normal_operand(node.operand[2]).as_constant().as_i32(),
//                         *self.normal_operand(node.operand[3]).as_register(),
//                     ))
//                 }
//                 MemNodeKind::BaseFiAlignOffOff => {
//                     MachineOperand::Mem(MachineMemOperand::BaseFiAlignOffOff(
//                         *self.normal_operand(node.operand[0]).as_register(),
//                         *self.normal_operand(node.operand[1]).as_frame_index(),
//                         self.normal_operand(node.operand[2]).as_constant().as_i32(),
//                         *self.normal_operand(node.operand[3]).as_register(),
//                         self.normal_operand(node.operand[4]).as_constant().as_i32(),
//                     ))
//                 }
//                 MemNodeKind::BaseFiOff => MachineOperand::Mem(MachineMemOperand::BaseFiOff(
//                     *self.normal_operand(node.operand[0]).as_register(),
//                     *self.normal_operand(node.operand[1]).as_frame_index(),
//                     self.normal_operand(node.operand[2]).as_constant().as_i32(),
//                 )),
//                 MemNodeKind::BaseOff => MachineOperand::Mem(MachineMemOperand::BaseOff(
//                     *self.normal_operand(node.operand[0]).as_register(),
//                     self.normal_operand(node.operand[1]).as_constant().as_i32(),
//                 )),
//                 MemNodeKind::AddressOff => MachineOperand::Mem(MachineMemOperand::AddressOff(
//                     inst::AddressKind::Global(*node.operand[0].as_address().as_global()),
//                     self.normal_operand(node.operand[1]).as_constant().as_i32(),
//                 )),
//                 MemNodeKind::AddressAlignOff => {
//                     MachineOperand::Mem(MachineMemOperand::AddressAlignOff(
//                         inst::AddressKind::Global(*node.operand[0].as_address().as_global()),
//                         self.normal_operand(node.operand[1]).as_constant().as_i32(),
//                         *self.normal_operand(node.operand[2]).as_register(),
//                     ))
//                 }
//                 MemNodeKind::Address => MachineOperand::Mem(MachineMemOperand::Address(match node
//                     .operand[0]
//                     .as_address()
//                 {
//                     node::AddressKind::Global(id) => inst::AddressKind::Global(*id),
//                     node::AddressKind::FunctionName(name) => {
//                         inst::AddressKind::FunctionName(name.clone())
//                     }
//                     node::AddressKind::Const(id) => inst::AddressKind::Constant(*id),
//                 })),
//             },
//             NodeKind::None => MachineOperand::None,
//             _ => MachineOperand::Register(self.convert(node).unwrap()),
//         }
//     }
// }
//
// pub fn mov_r_x(rc: RegisterClassKind, x: &MachineOperand) -> Option<MachineOpcode> {
//     let mov8rx = [MachineOpcode::MOVrr8, MachineOpcode::MOVri8];
//     let mov32rx = [MachineOpcode::MOVrr32, MachineOpcode::MOVri32];
//     let mov64rx = [MachineOpcode::MOVrr64, MachineOpcode::MOVri64];
//     let movsdrx = [MachineOpcode::MOVSDrr, MachineOpcode::MOVSDrm64];
//     let idx = match x {
//         MachineOperand::Register(_) => 0,
//         MachineOperand::Constant(_) => 1,
//         _ => return None,
//     };
//     match rc {
//         RegisterClassKind::GR8 => Some(mov8rx[idx]),
//         RegisterClassKind::GR32 => Some(mov32rx[idx]),
//         RegisterClassKind::GR64 => Some(mov64rx[idx]),
//         RegisterClassKind::XMM => Some(movsdrx[idx]),
//     }
// }
//
// pub fn mov_n_rx(bit: usize, x: &MachineOperand) -> Option<MachineOpcode> {
//     // TODO: refine code
//     assert!(bit > 0 && ((bit & (bit - 1)) == 0));
//
//     let mov32rx = [MachineOpcode::MOVrr32, MachineOpcode::MOVri32];
//     let xidx = match x {
//         MachineOperand::Register(_) => 0,
//         MachineOperand::Constant(_) => 1,
//         _ => return None, // TODO: Support Address?
//     };
//     match bit {
//         32 => Some(mov32rx[xidx]),
//         _ => None,
//     }
// }
//
// // TODO: Will be deprecated
// pub fn mov_rx(tys: &Types, regs_info: &RegistersInfo, x: &MachineOperand) -> Option<MachineOpcode> {
//     // TODO: special handling for float
//     if x.get_type(regs_info).unwrap() == Type::f64 {
//         return match x {
//             MachineOperand::Constant(_) => Some(MachineOpcode::MOVSDrm64),
//             MachineOperand::FrameIndex(_) | MachineOperand::Mem(_) => Some(MachineOpcode::MOVSDrm),
//             MachineOperand::Register(_) => Some(MachineOpcode::MOVSDrr),
//             _ => None,
//         };
//     }
//
//     let mov8rx = [MachineOpcode::MOVrr8];
//     let mov32rx = [
//         MachineOpcode::MOVrr32,
//         MachineOpcode::MOVri32,
//         MachineOpcode::MOVrm32,
//     ];
//     let mov64rx = [
//         MachineOpcode::MOVrr64,
//         MachineOpcode::MOVri64,
//         MachineOpcode::MOVrm64,
//     ];
//     let (bit, xidx) = match x {
//         MachineOperand::Register(r) => {
//             if let Some(sub_super) = r.sub_super {
//                 (sub_super.size_in_bits(), 0)
//             } else {
//                 (regs_info.arena_ref()[r.id].reg_class.size_in_bits(), 0)
//             }
//         }
//         MachineOperand::Constant(c) => (c.size_in_bits(), 1),
//         MachineOperand::Mem(m) => (m.get_type().unwrap().size_in_bits(tys), 2),
//         _ => return None, // TODO: Support Address?
//     };
//     match bit {
//         8 => Some(mov8rx[xidx]),
//         32 => Some(mov32rx[xidx]),
//         64 => Some(mov64rx[xidx]),
//         _ => None,
//     }
// }
//
// pub fn mov_mx(regs_info: &RegistersInfo, x: &MachineOperand) -> Option<MachineOpcode> {
//     if x.get_type(regs_info).unwrap() == Type::f64 {
//         return match x {
//             MachineOperand::Register(_) => Some(MachineOpcode::MOVSDmr),
//             _ => None,
//         };
//     }
//
//     let mov32mx = [MachineOpcode::MOVmr32, MachineOpcode::MOVmi32];
//     let mov64mx = [MachineOpcode::MOVmr64, MachineOpcode::MOVmi64];
//     // let mov64rx = [
//     //     MachineOpcode::MOVrr64,
//     //     MachineOpcode::MOVri64,
//     //     MachineOpcode::MOVrm64,
//     // ];
//     let (bit, n) = match x {
//         MachineOperand::Register(r) => {
//             if let Some(sub_super) = r.sub_super {
//                 (sub_super.size_in_bits(), 0)
//             } else {
//                 (regs_info.arena_ref()[r.id].reg_class.size_in_bits(), 0)
//             }
//         }
//         MachineOperand::Constant(c) => (c.size_in_bits(), 1),
//         _ => return None, // TODO: Support Address?
//     };
//     match bit {
//         32 => Some(mov32mx[n]),
//         64 => Some(mov64mx[n]),
//         _ => None,
//     }
// }
