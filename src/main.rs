#![feature(rustc_private)]

// NOTE: For the example to compile, you will need to first run the following:
//   rustup component add rustc-dev llvm-tools-preview

// version: rustc 1.68.0-nightly (935dc0721 2022-12-19)

mod util;

extern crate rustc_error_codes;
extern crate rustc_errors;
extern crate rustc_hash;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_session;
extern crate rustc_span;
extern crate rustc_driver;
extern crate rustc_middle;

use rustc_driver::Compilation;
use rustc_interface::{interface, Queries};
use rustc_middle::ty::TyCtxt;
use util::{show_items, list_unsafe_blocks_in_fn};


struct MyCallbacks;
impl rustc_driver::Callbacks for MyCallbacks {
    fn after_analysis<'tcx>(
        &mut self,
        compiler: &interface::Compiler,
        queries: &'tcx Queries<'tcx>,
    ) -> Compilation {
        queries
            .global_ctxt()
            .unwrap()
            .enter(|tcx| self.demo(compiler, tcx));

        Compilation::Continue
    }
}

impl MyCallbacks {
    fn demo<'tcx>(&mut self, compiler: &interface::Compiler, tcx: TyCtxt<'tcx>) {
        for def_id in util::list_functions(tcx) {
            let unsafe_spans = list_unsafe_blocks_in_fn(def_id, tcx);
            let fns = util::list_functions(tcx);
            for def_if in fns {
                let unsafe_blocks = util::list_unsafe_blocks_in(def_if, &unsafe_spans, tcx);
                println!("{:?}\n\n", unsafe_blocks);
            }
        }
        // 
        // for def_id in fns {
        //     // let hir_id = map.get_by_def_id(local_def_id).body_id().unwrap().hir_id;
        //     util::show_basic_blocks(def_id, tcx);
        // }
    }
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let mut my_cb = MyCallbacks;
    rustc_driver::RunCompiler::new(&args, &mut my_cb).run();
}