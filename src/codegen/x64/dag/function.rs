use super::super::register::*;
use super::{basic_block::*, frame_object::*, node::*};
use crate::ir::{function::*, types::*};
use crate::util::allocator::*;
use id_arena::*;

pub type DAGFunctionId = Id<DAGFunction>;

pub struct DAGFunction {
    /// Function name
    pub name: String,

    /// Function type
    pub ty: Type,

    /// DAG Basic block arena
    pub dag_basic_block_arena: Arena<DAGBasicBlock>,

    /// DAG Basic blocks list
    pub dag_basic_blocks: Vec<DAGBasicBlockId>,

    /// DAG node arena
    pub dag_heap: RawAllocator<DAGNode>,

    // /// True if internal function
    // pub internal: bool,
    pub local_mgr: LocalVariableManager,

    /// Virtual register generator
    pub vreg_gen: VirtRegGen,
}

impl DAGFunction {
    pub fn new(
        func: &Function,
        dag_heap: RawAllocator<DAGNode>,
        dag_basic_block_arena: Arena<DAGBasicBlock>,
        dag_basic_blocks: Vec<DAGBasicBlockId>,
        local_mgr: LocalVariableManager,
        vreg_gen: VirtRegGen,
    ) -> Self {
        Self {
            name: func.name.clone(),
            ty: func.ty.clone(),
            dag_basic_block_arena,
            dag_basic_blocks,
            dag_heap,
            local_mgr,
            vreg_gen,
        }
    }
}
