# 内存管理

- `rustc`使用一块大内存，编译过程中产生的数据结构都存在这块内存中，这样的数据结构称为`arena-allocated data structures`。这块内存的生命周期是`'tcx`。所以，拥有`'tcx`生命周期的数据，就和这块内存活得一样久。
- [`tcx: TyCtxt<'tcx> ` ](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/ty/struct.TyCtxt.html)("typing context") 是编译过程中的核心数据结构，里面存储了：
  - 各种其他arena数据的引用
  - 各种queries结果的缓存
- 编译器中使用了很多Thread-local来存储数据。[`rustc_middle::ty::tls`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/ty/tls/index.html) 模块用来访问这些Thread-local。



# 编译过程

## 入口

- `rustc_driver`是编译过程的总控制器，相当于编译器的`main()`方法。就像胶水一样把编译的各个阶段粘在一起。
- `rustc_interafce`提供了控制具体编译阶段的api。`rustc_driver`也是通过调用这些api来控制编译过程的。
- `rustc_driver::RunCompiler`结构体作为编译入口。new时传入：
  - 一些选项参数。
  - `rustc_driver::Callbacks`特征对象，用于在不同阶段执行对应的代码。

## Callbacks

- 文档：[`rustc_driver::Callbacks`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_driver/trait.Callbacks.html)

- 这个特征中提供了4个方法，分别对应在编译的不同时刻的回调方法，我们可以按需要进行实现。

  - ```rust
    // compiler实例创建前
    fn config(&mut self, _config: &mut Config)
    // parsing后
    fn after_parsing<'tcx>(&mut self, _compiler: &Compiler, _queries: &'tcx Queries<'tcx>) -> Compilation
    // expansion后
    fn after_expansion<'tcx>(&mut self, _compiler: &Compiler, _queries: &'tcx Queries<'tcx>) -> Compilation
    // analysis后
    fn after_analysis<'tcx>(&mut self, _compiler: &Compiler, _queries: &'tcx Queries<'tcx>) -> Compilation
    ```

    其中，后三个`after_*`方法都返回一个[rustc_driver::Compilation](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_driver/enum.Compilation.html#)，表示继续或是停止编译。

    - ```rust
      pub enum Compilation {Stop, Continue}
      ```

- 初步调研发现，各种分析工具似乎都只实现了`config()`和`after_analysis()`。

## 具体过程

- 对我们创建的`Runompiler`结构调用`RunCompiler::run()`方法。该方法又调用私有方法`RunCompiler::run_compiler()`。

- `run_compiler()`中：

  - 解析选项和命令行参数等

  - 调用 `rustc_interface::run_compiler()`方法。该方法接受：

    - `rustc_interface::interface::Config`结构体，包含各种选项参数
    - 一个闭包。这个闭包的参数是一个`rustc_interface::interface::Compiler`结构（表示一个compiler session，大概是指一个编译过程的实体，包含了有关这次编译的各种信息。如`codegen_backend`以及各种queries）。在这个闭包中，我们可以使用这个Compiler里的queries等来驱动编译过程，`rustc_driver`也是这样做的。

    `rustc_interface::run_compiler()`方法会根据传入的`Config`新建一个`Compiler`，然后将它作为参数传入closure。

    - `rustc_interface::run_compiler()`中，又调用了`Compiler::enter()`：
  
      - ```rust
        pub fn enter<F, T>(&self, f: F) -> T 
        	where F: for<'tcx> FnOnce(&'tcx Queries<'tcx>) -> T
        ```
  
        它接受一个闭包，这个闭包又以一个`rustc_interface::Queries`为参数。
  
        它用`slef`新建一个`Queries`，然后将新建的`Queries`传入自己接受的闭包参数。
  
  - `RunCompiler::run_compiler()`方法中：
  
    - 处理输入参数选项
    - 调用`callback.config()`
    - 调用`rustc_interface::run_compiler()`，其中传入的闭包（参数记为`queries`）做了这些事情：
      - 调用`queries.parse()`，parse到AST？
        - 调用`callbacks.after_parsing()`，返回`Continue`则继续，返回`Stop`则停止。
      - 。。。
      - 调用`callbacks.after_expansion()`，。。。
      - 。。。
      - 调用`callbacks.after_analysis()`，。。。

# 中间表示

- https://play.rust-lang.org/?version=stable&mode=debug&edition=2021：在线编辑器，能打印出rust代码在编译过程中产生的各种中间表示。

## HIR

- HIR部分的顶层数据结构是[`rustc_hir::Crate`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_hir/struct.Crate.html)，但似乎不怎么直接使用它。

- HIR中的各种节点，有一个枚举[`rustc_hir::hir::Node`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_hir/hir/enum.Node.html)来表示。里面的各种字段都含有`span`（源码中的位置，行，列等），以及一个ID。但ID的类型不一定。

  - `Node`中最重要的是`Item`。它大概是指一个crate中的“顶级”（最外层？）元素，如`use, struct定义， fn定义， mod定义等`。[源码注释](https://doc.rust-lang.org/nightly/nightly-rustc/src/rustc_hir/hir.rs.html#3033)中说：`Items are always HIR owners.`

  - ```rust
    // rustc_hir::hir::Item
    pub struct Item<'hir> {
        ...
        pub kind: ItemKind<'hir>
        ...
    }
    
    // rustc_hir::ItemKind
    // 注意这里的ItemKind是HIR Node的类型，不要与rustc_middle::ty::Ty搞混。后者是rust语言类型系统中的类型。
    pub enum ItemKind<'hir> {
        ExternCrate(Option<Symbol>),
        Use(&'hir UsePath<'hir>, UseKind),
        Static(&'hir Ty<'hir>, Mutability, BodyId),
        Const(&'hir Ty<'hir>, BodyId),
        Fn(FnSig<'hir>, &'hir Generics<'hir>, BodyId),
        Macro(MacroDef, MacroKind),
        Mod(&'hir Mod<'hir>),
        ForeignMod {
            abi: Abi,
            items: &'hir [ForeignItemRef],
        },
        GlobalAsm(&'hir InlineAsm<'hir>),
        TyAlias(&'hir Ty<'hir>, &'hir Generics<'hir>),
        OpaqueTy(OpaqueTy<'hir>),
        Enum(EnumDef<'hir>, &'hir Generics<'hir>),
        Struct(VariantData<'hir>, &'hir Generics<'hir>),
        Union(VariantData<'hir>, &'hir Generics<'hir>),
        Trait(IsAuto, Unsafety, &'hir Generics<'hir>, GenericBounds<'hir>, &'hir [TraitItemRef]),
        TraitAlias(&'hir Generics<'hir>, GenericBounds<'hir>),
        Impl(&'hir Impl<'hir>),
    }
    ```

- 大部分时候，我们其实是在与[`rustc_middle::hir::map::Map`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/hir/map/struct.Map.html)交互。`tcx.hir()`返回的即是这个`Map`结构。

  - `Map`中，我们能直接获取到`items`。

    [`TyCtxt::hir_crate_items()`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/ty/context/struct.TyCtxt.html#method.hir_crate_items)或[`Map::items()`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/hir/map/struct.Map.html#method.items)可以获取当前正在编译的crate中所有的`item`的`ItemId`。再通过`TyCtxt::find(Itemid)`或`TyCtxt::get(ItemId)`来获取`rustc_hir::hir::Node`，这是一个枚举，其中包括`Item`，但我们一开始获取的就是`items`，所以直接匹配即可。代码参见`utils::show_types`。

  - `Map`中还定义了各种根据ID获取item，以及在ID之间转换的方法。参见：https://rustc-dev-guide.rust-lang.org/hir.html#the-hir-map

  - 各种ID的定义，参见https://rustc-dev-guide.rust-lang.org/identifiers.html


## THIR

todo

update：从没有见人用过。

## MIR

- high-level的介绍，以及一些相关的概念：https://blog.rust-lang.org/2016/04/19/MIR.html
- https://projekter.aau.dk/projekter/files/421583418/Static_Taint_Analysis_in_Rust.pdf：Chapter 5是对MIR语法和语义的形式化描述。
- MIR的语法不再赘述了，这里记录几个常见概念和API:
  - todo


#### 重要概念和API

- MIR是基于CFG的。表达式类似三地址码，不能嵌套。类型解析和检查在生成MIR之前完成，所以MIR中可以访问完整的类型信息。
- [`rustc_middle::mir::Body`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/struct.Body.html#)：表示一个方法体，但也有参数和返回值的信息。文档比较详细。注意：
  - `arg_count`字段是入参数量。
  - `local_decls`字段是方法中所有局部变量。其中第一个是返回值，然后是`arg_count`个入参。
- 要访问基本块（bb），访问`Body::basic_blocks`字段，或调用`Body::basic_blocks_mut()`。主要说一下前者：
  - 该字段是一个[`rustc_middle::mir::basic_blocks::BasicBlocks`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/basic_blocks/struct.BasicBlocks.html)，其实是一个从`rustc_middle::mir::BasicBlock`到`rustc_middle::mir::BasicBlockData`的表。其中前者只是一个ID，后者才真正包含了bb中的数据。
  - 该字段中定义了各种方法，如遍历，判断CFG中是否有环，寻找一个节点的`predecessors`和`dominators`等。
- 在一个`BasicBlockData`中：
  - `BasicBlockData::local_decls`字段是一个`rustc_middle::mir::Local`到[`rustc_middle::mir::LocalDecl`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/struct.LocalDecl.html)的表，存储局部变量。局部变量可以是用户定义的变量，临时变量，入参，返回值。
    - `rustc_middle::mir::Local`只含有一个ID，[`rustc_middle::mir::LocalDecl`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/struct.LocalDecl.html)表示真正的变量信息。
  - 类似LLVM IR，每个bb都有一个`terminator`。参见[`rustc_middle::mir::terminator::Terminator`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/terminator/struct.Terminator.html)。
  - 一条语句用[`rustc_middle::mir::Statement`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/struct.Statement.html)表示。[`rustc_middle::mir::syntax::StatementKind`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/syntax/enum.StatementKind.html)页面有对各种语句的说明。
- [`rustc_middle::mir`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/index.html#constants)中定义了几个有用的常量：
  - [`rustc_middle::mir::RETURN_PLACE`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/constant.RETURN_PLACE.html)：代表返回值的Local。（其实就是0，可以用一个`Local`和它比较。
  - [`rustc_middle::mir::START_BLOCK`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/constant.START_BLOCK.html)：代表入口bb。（也是0，可以直接用一个`BasicBlock`和它比较。

#### 遍历

- 可以直接用`rustc_middle::mir::basic_blocks::BasicBlocks`中的 [`postorder`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/basic_blocks/struct.BasicBlocks.html#method.postorder)等方法获取某个顺序的遍历，返回的都是`BasicBlock`引用数组。

- visitor特征定义在[`rustc_middle::mir::visit`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/visit/index.html)中。visitor有两种：
  - [`rustc_middle::mir::visit::Visitor`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/visit/trait.Visitor.html)：operates on a `&Mir` and gives back shared references，只读取，不改变。我们只做分析的话应该够用？
  - [`rustc_middle::mir::visit::MutVisitor`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/visit/trait.MutVisitor.html)：operates on a `&mut Mir` and gives back mutable references，可以改变。
- 两种Visitor特征中都定义了各种`visit_*`方法，如`visit_local(), visit_statement() `等，用于在访问到对应的结构时调用。
- [`rustc_middle::mir::traversal`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/traversal/index.html)中定义了遍历相关的API，如[`Postorder`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/traversal/struct.Postorder.html)，[`Preorder`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/traversal/struct.Preorder.html)等。
- 例子：https://doc.rust-lang.org/nightly/nightly-rustc/src/rustc_mir_transform/const_debuginfo.rs.html

# 备忘

- Compiler::enter

  - ```rust
    pub fn enter<F, T>(&self, f: F) -> T
        where
            F: for<'tcx> FnOnce(&'tcx Queries<'tcx>) -> T,
    ```

  - 用slef新建一个q::Queries

  - f(q)





# Getting Started

首先切换到`rust-nightly`，因为`rustc`开发所需的相关接口都只能在不稳定版本（nightly）里使用。

Rust 有三个**发布通道（release channels）**：

- 每夜版（Nightly）每天晚上更新的不稳定版本
- 测试版（Beta）
- 稳定版（Stable）

```shell
# 安装
rustup toolchain install nightly
# 切换到nightly
rustup default nightly
# 切换回stable
rustup default stable

# 或者，在某个项目根目录下执行以下命令，那么仅该项目会默认使用nightly
rustup override set nightly
```



安装`rustc`开发所需工具

```shell
rustup component add rustc-dev llvm-tools-preview
```



入口文件开头加入：

```rust
#![feature(rustc_private)] // 使用rustc相关接口
#![deny(rustc::internal)]
extern crate rustc_driver; // 引入相关crate，仅需在项目入口文件处 extern crate ...
extern crate rustc_interface;
extern crate rustc_errors;
extern crate rustc_lint;
```



# Design

## 在MIR中标记unsafe代码块

- MIR中本身不含有unsafe信息，但是HIR中有（`rustc_hir::hir::Block::rules`字段）。
- HIR的`Block`和MIR`Stmt`中都有`span`。我们先在HIR中记录unsafe块的span，然后在MIR中筛选。
  - 每个MIR块中拿一个`Stmt`出来，如果它的span范围在某个之前记录的unsafe HIR块内，那么它所在的这个块是unsafe的。
