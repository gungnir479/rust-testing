use rustc_middle::{
    ty,
    ty::{Ty, TyCtxt, DefIdTree, TyKind},
    hir::map::Map,
    mir::{Body, BasicBlock, BasicBlockData},
};
use rustc_hir::{
    self,
    {Node, ItemKind, Expr, ExprKind, Block, Stmt, StmtKind},
    def::DefKind,
    def_id::{DefId, DefIndex, LocalDefId},
    hir_id::HirId,
};
use rustc_span::Span;
use rustc_span::symbol::Ident;
use std::collections::HashSet;


fn log(content: &str) {
    println!("=================== {} ===================\n", content);
}


pub fn local_def_id_to_hir_id(local_def_id: LocalDefId, tcx: TyCtxt) -> HirId {
    tcx.hir().local_def_id_to_hir_id(local_def_id)
}


pub fn def_id_to_local_def_id(def_id: DefId) -> LocalDefId {
    def_id.expect_local()
}


pub fn def_id_to_hir_id(def_id: DefId, tcx: TyCtxt) -> HirId {
    local_def_id_to_hir_id(def_id_to_local_def_id(def_id), tcx)
}


pub fn expect_bolck(hir_id: HirId, tcx: TyCtxt) -> &Block {
    return match tcx.hir().find(hir_id) {
        Some(Node::Block(b)) => b,
        _ => panic!("expected expr, found {}", tcx.hir().node_to_string(hir_id)),
    };
}


pub fn expect_stmt(hir_id: HirId, tcx: TyCtxt) -> &Stmt {
    return match tcx.hir().find(hir_id) {
        Some(Node::Stmt(s)) => s,
        _ => panic!("expected expr, found {}", tcx.hir().node_to_string(hir_id)),
    };
}


// show the type of each item in the ctate.
pub fn show_types<'tcx>(tcx: TyCtxt) {
    let hir: Map = tcx.hir();
    for item_id in hir.items() {
        let hir_id: HirId = item_id.hir_id();
        let node: Node = hir.find(hir_id).unwrap();
        if let Node::Item(item) = node {
            println!("{:?}", item.kind.descr());
        }
    }
}

// show all the functions in a crate and return their DefId in a Vec.
pub fn list_functions<'tcx>(tcx: TyCtxt) -> Vec<DefId>{
    log("list_functions");
    // let entry_fn_def_id = if let Some((def_id, _)) = tcx.entry_fn(()) {
    //     def_id
    // } else {
    //     DefId::local(DefIndex::from_u32(0))
    // };
    let mut res: Vec<DefId> = Vec::new();
    for local_def_id in tcx.hir().body_owners() {
        let def_id: DefId = local_def_id.to_def_id();
        let name = tcx.item_name(def_id);
        let kind: DefKind = tcx.def_kind(def_id);
        if kind == DefKind::Fn || kind == DefKind::AssocFn {
            // println!("{}", name);
            res.push(def_id);
        }
    }
    res
}


// Returns true if the function identified by def_id is defined as part of a trait.
pub fn is_trait_method(def_id: DefId, tcx: TyCtxt<'_>) -> bool {
    log("is_trait_method");
    tcx.is_trait(tcx.parent(def_id))
}


// Given an Adt(struct, enum, union) type, list all the type of its varients.
pub fn list_varient_types<'tcx>(ty: Ty<'tcx>, tcx: TyCtxt<'tcx>) {
    log("list_varient_types");

    if let TyKind::Adt(def, substs) = ty.kind() {
        for variant in def.variants().iter() {
            for (_, field) in variant.fields.iter().enumerate() {
                let field_ty: Ty = field.ty(tcx, substs);
                let symbol: Ident = field.ident(tcx);
                println!("{}: {}", symbol, field_ty);
            }
        }
    }
    panic!("Need an Adt type!");
}


// Returns true if the given type is a function, a closure, a generator, or a struct with
// a field that is (or contains) a function in this sense.
// This does not traverse references, so the answer is approximate.
pub fn contains_function<'tcx>(ty: Ty<'tcx>, tcx: TyCtxt<'tcx>) -> bool {
    log("contains_function");

    if ty.is_fn() || ty.is_closure() || ty.is_generator() {
        return true;
    }

    if let TyKind::Adt(def, substs) = ty.kind() {
        for variant in def.variants().iter() {
            for (_, field) in variant.fields.iter().enumerate() {
                let field_ty = field.ty(tcx, substs);
                if contains_function(field_ty, tcx) {
                    return true;
                }
            }
        }
    }
    false
}


// Given the DefId of a function, show all basic blocks in the function.
// Attention: input DefId is the id of the function body's owner, rather than the body itself. (DefId from list_functions)
pub fn show_basic_blocks<'tcx>(def_id: DefId, tcx: TyCtxt<'tcx>) {
    log("show_basic_blocks");
    // let def_id_ = tcx.hir().get_if_local(def_id).unwrap().expect_item().owner_id.to_def_id();
    // assert_eq!(def_id, def_id_);
    let body = tcx.optimized_mir(def_id);

    println!("{}\n", tcx.item_name(def_id));
    for (bb, bbData) in body.basic_blocks.iter_enumerated() {
        show_statements(bbData);
    }
}


// Show all the statements in a given basic block.
pub fn show_statements(basic_block_data: &BasicBlockData) {
    for stmt in &basic_block_data.statements {
        println!("{:?}\n\n", stmt.kind);
    }
}


pub fn show_items<'tcx>(tcx: TyCtxt<'tcx>) {
    println!("{:?}", tcx.hir_crate(()));
    // let map = tcx.hir();
    // for item_id in map.items() {
    //     println!("{:?}", map.item(item_id));
    // }
}


// Return all unsafe blocks in a HIR function.
pub fn list_unsafe_blocks_in_fn<'tcx, 'hir>(def_id: DefId, tcx: TyCtxt<'tcx>) -> Vec<Span> where 'tcx: 'hir {
    let mut rst: Vec<Span> = vec![];
    let map = tcx.hir();
    let local_def_id = def_id_to_local_def_id(def_id);
    if let ItemKind::Fn(_, _, body_id) = map.expect_item(local_def_id).kind {
        let body = map.body(body_id);
        for hir_id in list_exprs_in_expr(body.value.hir_id, tcx) {
            if let ExprKind::Block(b, _) = map.expect_expr(hir_id).kind {
                if let Block{rules: rustc_hir::BlockCheckMode::UnsafeBlock(us), ..} = b {
                    rst.push(b.span);
                }
            }
        };
    }
    rst
}


// return all exprs in a stmt.
pub fn list_exprs_in_stmt<'tcx, 'hir>(hir_id: HirId, tcx: TyCtxt<'tcx>) -> HashSet<HirId> where 'tcx: 'hir {
    // println!("{:?}\n", hir_id);
    let stmt = expect_stmt(hir_id, tcx);
    let exprs = match stmt.kind {
        StmtKind::Local(l) => {
            let mut tmp = HashSet::new();
            if let Some(e) = l.init { tmp = &tmp | &list_exprs_in_expr(e.hir_id, tcx); };
            if let Some(b) = l.els { tmp = &tmp | &list_exprs_in_expr(b.hir_id, tcx); };
            tmp
        },
        StmtKind::Item(_) => HashSet::new(),
        StmtKind::Expr(e) => list_exprs_in_expr(e.hir_id, tcx),
        StmtKind::Semi(e) => list_exprs_in_expr(e.hir_id, tcx)
    };
    
    exprs
}


// return all exprs within an expr, including itself and its sub exprs.
pub fn list_exprs_in_expr<'tcx, 'hir>(hir_id: HirId, tcx: TyCtxt<'tcx>) -> HashSet<HirId> where 'tcx: 'hir {
    let expr = tcx.hir().expect_expr(hir_id);
    let sub_exprs = match expr.kind {
        ExprKind::Box(e) => vec![e],
        ExprKind::ConstBlock(ac) => vec![tcx.hir().body(ac.body).value],
        ExprKind::Array(ea) => {let mut tmp = vec![]; for e in ea { tmp.push(e) }; tmp},
        ExprKind::Call(e, ea) => {let mut tmp = vec![e]; for e in ea { tmp.push(e) }; tmp},
        ExprKind::MethodCall(_, e, ea, _) => {let mut tmp = vec![e]; for e in ea { tmp.push(e) }; tmp},
        ExprKind::Tup(ea) => {let mut tmp = vec![]; for e in ea { tmp.push(e) }; tmp},
        ExprKind::Binary(_, e1, e2) => vec![e1, e2],
        ExprKind::Unary(_, e) => vec![e],
        ExprKind::Lit(_) => vec![],
        ExprKind::Cast(e, _) => vec![e],
        ExprKind::Type(e, _) => vec![e],
        ExprKind::DropTemps(e) => vec![e],
        ExprKind::Let(l) => vec![l.init],
        ExprKind::If(e1, e2, op) => match op {Some(e) => vec![e1, e2, e], None => vec![e1, e2]},
        ExprKind::Loop(b, ..) => vec![], //list_exprs_in_block(b, tcx),
        ExprKind::Match(e, arm, _) => {let mut res = vec![e]; for a in arm {res.push(a.body)} res},
        ExprKind::Closure(c) => vec![tcx.hir().body(c.body).value],
        ExprKind::Block(b, _) => vec![], //list_exprs_in_block(b, tcx),
        ExprKind::Assign(e1, e2, _) => vec![e1, e2],
        ExprKind::AssignOp(_, e1, e2) => vec![e1, e2],
        ExprKind::Field(e, _) => vec![e],
        ExprKind::Index(e1, e2) => vec![e1, e2],
        ExprKind::Path(_) => vec![],
        ExprKind::AddrOf(_, _, e) => vec![e],
        ExprKind::Break(..) => vec![],
        ExprKind::Continue(..) => vec![],
        ExprKind::Ret(oe) => match oe {Some(e) => vec![e], None => vec![]},
        // todo
        ExprKind::InlineAsm(..) => vec![],
        ExprKind::Struct(_, efa, oe) => {let mut tmp = vec![]; for ef in efa { tmp.push(ef.expr) }; if let Some(e) = oe {tmp.push(e)}; tmp},
        ExprKind::Repeat(e, _) => vec![e],
        ExprKind::Yield(e, _) => vec![e],
        ExprKind::Err => vec![],
    };

    let mut sub_hir_ids: HashSet<HirId> = &HashSet::from([hir_id]) | &match expr.kind {
        ExprKind::Loop(b, ..) => list_exprs_in_block(b.hir_id, tcx),
        ExprKind::Block(b, _) => list_exprs_in_block(b.hir_id, tcx),
        _ => vec![].into_iter().collect()
    };

    for e in sub_exprs {
        sub_hir_ids.insert(e.hir_id);
        sub_hir_ids = &sub_hir_ids | &list_exprs_in_expr(e.hir_id, tcx);
    }
    // println!("{:?} {:?} {:?}\n\n", expr.kind, expr.span, sub_hir_ids.len());
    sub_hir_ids
}


// return all blocks within an expr.
pub fn list_exprs_in_block<'tcx, 'hir>(hir_id: HirId, tcx: TyCtxt<'tcx>) -> HashSet<HirId> where 'tcx: 'hir {
    let block = expect_bolck(hir_id, tcx);
    let mut rst: HashSet<HirId> = HashSet::new();
    for stmt in block.stmts {
        rst = &rst | &list_exprs_in_stmt(stmt.hir_id, tcx);
    }
    if let Some(expr) = block.expr { rst = &rst | &list_exprs_in_expr(expr.hir_id, tcx); };
    rst
}


// return all unsafe blocks in a function.
pub fn list_unsafe_blocks_in<'tcx>(def_id: DefId, unsafe_spans: &Vec<Span>, tcx: TyCtxt<'tcx>) -> Vec<BasicBlock> {
    let mut rst = vec![];
    let body = tcx.optimized_mir(def_id);
    for (bb, bbData) in body.basic_blocks.iter_enumerated() {
        if bbData.statements.iter().any(|s| {
            unsafe_spans.iter().any(|&span| s.source_info.span <= span)
        }) {
            rst.push(bb);
        }
    }
    rst
}