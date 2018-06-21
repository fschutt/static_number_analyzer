extern crate syn;
extern crate quote; // only used for debugging / dumping as text

use syn::synom::ParseError;
use syn::{Item, ItemFn, FnDecl, Block, BinOp, parse_file};
use std::collections::HashMap;
use std::ops::Range;

mod range_comp;

const TEST_STRING: &str = include_str!("./test.rs");

#[derive(Debug, Clone)]
struct Dependency {
    // What variable this depends on
    dependent_on: String,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct LetStatement {
    variable_name: String,
    // position: Span,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum LetInitialization {
    Range(Range<usize>),
    Dependency(String),
}

impl LetInitialization {
    fn as_range(&self) -> Option<Range<usize>> {
        use self::LetInitialization::*;
        match self {
            Range(r) => Some(r.clone()),
            Dependency(_) => None,
        }
    }
}

#[derive(Debug, Clone)]
struct IfExpression {
    var1: String,
    cond: BinOp,
    var2: String,
    // position: Span,
}

fn main() {
    for function in break_file_into_functions(TEST_STRING).unwrap().into_iter() {
        check_function(function);
    }
}

// Parse a rust source file and break it into functions
fn break_file_into_functions(file: &str) -> Result<Vec<ItemFn>, ParseError> {

    let file = parse_file(file)?;

    let mut functions = Vec::<ItemFn>::new();

    for item in file.items.into_iter() {
        match item {
            Item::Fn(f) => {
                functions.push(f)
            },
            // mod - may contain impl, which contain fns
            // impl - may contain fns
            _ => { },
        }
    }

    Ok(functions)
}

fn check_function(func: ItemFn) {

    let function_name = func.ident.to_string();
    let variable_sources = parse_function_arguments(&func.decl);
    let internal_let_statements = parse_let_statements(&func.block);
    let if_conditions = parse_if_conditions(&func.block);

    check_for_illogical_ranges(&function_name, &variable_sources, &internal_let_statements, &if_conditions);
    check_for_illogical_dependencies(); // TODO
}

/// Parses possible sources of variables passed into a function via inputs:
///
/// `fn(a: usize, b: usize)` -> `["a", (USIZE_MIN..USIZE_MAX), "b", (USIZE_MIN..USIZE_MAX)]`
fn parse_function_arguments(function_args: &FnDecl) -> HashMap<String, Range<usize>> {

    use syn::{FnArg, Pat, ArgCaptured, PatIdent, Type, TypePath, Path};
    use std::usize::{MIN as USIZE_MIN, MAX as USIZE_MAX};

    let mut variable_sources = HashMap::<String, Range<usize>>::default();

    // TODO: do this recursively, currently this will only catch the outermost if statement

    for source in function_args.inputs.iter() {
        if let FnArg::Captured(ArgCaptured {
            pat: Pat::Ident(PatIdent { ident, .. }),
            ty: Type::Path(TypePath {
                path: Path { segments, .. },
                ..
            }),
            ..
        }) = source
        {
            let variable_name = ident.to_string();
            let variable_type = match segments.iter().last() {
                None => continue,
                Some(t) => t.ident.to_string(),
            };

            // If we get an a: usize, we assume that the variable a can
            // have any value from USIZE_MIN to USIZE_MAX
            match variable_type.as_ref() {
                "usize" => { variable_sources.insert(variable_name, USIZE_MIN..USIZE_MAX); },
                _ => { } // TODO
            }
        }
    }

    variable_sources
}

/// `let a = 5;` -> ["a @ line 1; LetInitialization::Range(5..5)"]
fn parse_let_statements(block: &Block) -> HashMap<LetStatement, LetInitialization> {
    use syn::{Stmt, Local, token::Let, ExprPath, Path, Pat, Lit, Expr, ExprLit};

    let mut let_statements = HashMap::default();

    for stmt in &block.stmts {
        if let Stmt::Local(Local { let_token: Let(_), pats, init, .. }) = stmt {

            let local_variable_name = match pats.iter().last() {
                Some(Pat::Ident(t)) => LetStatement { variable_name: t.ident.to_string() },
                _ => continue,
            };

            let initial_value = match init.as_ref().and_then(|i| Some(&*i.1)) {
                Some(Expr::Lit(ExprLit { lit: Lit::Int(l), .. })) => {
                    let l = l.value() as usize; // warn: cast
                    LetInitialization::Range(l..l)
                },
                Some(Expr::Path(ExprPath { path: Path { segments, .. }, .. })) => {
                    match segments.iter().last() {
                        Some(t) => LetInitialization::Dependency(t.ident.to_string()),
                        None => continue,
                    }
                },
                _ => continue,
            };

            let_statements.insert(local_variable_name, initial_value);
        }
    }

    let_statements
}

/// `if a < b` -> [IfExpression { var1: "a", cond: SmallerThan, var2: "b", location: "line 5" }]
fn parse_if_conditions(block: &Block) -> Vec<IfExpression> {
    use syn::{Stmt, Expr, ExprIf, ExprPath, Path};

    let mut conditions = Vec::new();

    for stmt in &block.stmts {
        if let Stmt::Expr(Expr::If(ExprIf { cond, .. })) = stmt {
            if let Expr::Binary(ref b) = **cond {
                let var_a = if let Expr::Path(ExprPath { path: Path { ref segments, .. }, .. }) = *b.left {
                    match segments.iter().last() {
                        None => continue,
                        Some(t) => t.ident.to_string()
                    }
                } else {
                    continue;
                };

                let var_b = if let Expr::Path(ExprPath { path: Path { ref segments, .. }, .. }) = *b.right {
                    match segments.iter().last() {
                        None => continue,
                        Some(t) => t.ident.to_string()
                    }
                } else {
                    continue;
                };

                let op = b.op;

                conditions.push(IfExpression { var1: var_a, cond: op, var2: var_b });
            }
        }
    }

    conditions
}

/// Prints a warning if certain things are "always true" or "always false"
///
/// I.e. `let a = 5; let b = 10; if a < b { } // always true`
fn check_for_illogical_ranges(
    f: &str,
    func_args: &HashMap<String, Range<usize>>,
    let_stmts: &HashMap<LetStatement, LetInitialization>,
    if_exprs: &[IfExpression])
{
    use syn::BinOp::*;
    use range_comp::RangeExt;

    for IfExpression { var1, cond, var2 } in if_exprs {
        use range_comp::RangeComparisonResult::*;

        let range_var1 = resolve_dependency(&LetInitialization::Dependency(var1.clone()), let_stmts, func_args);
        let range_var2 = resolve_dependency(&LetInitialization::Dependency(var2.clone()), let_stmts, func_args);
        let range_comparison = range_var1.compare(&range_var2);

        let text = match range_comparison {
            AlwaysLarger => {
                match cond {
                    Ge(_) | Gt(_) | Ne(_) => { "true" },
                    Eq(_) | Lt(_) | Le(_) => { "false" },
                    _ => { continue; },
                }
            },
            AlwaysEqual => {
                match cond {
                    Eq(_) | Ge(_) | Le(_) => { "true" },
                    Gt(_) | Ne(_) | Lt(_) => { "false" },
                    _ => { continue; },
                }
            },
            AlwaysSmaller => {
                continue;
            },
            RangeOverlaps => {
                continue;
            }
        };

        warn(f, var1, cond, var2, text);
    }
}

fn warn(func_name: &str, var1: &str, op: &BinOp, var2: &str, always: &str) {
    println!("WARNING: in fn {}: {} {} {}: is always {}", func_name, var1, cond_to_string(op), var2, always);
}

fn cond_to_string(op: &BinOp) -> &'static str {
    use syn::BinOp::*;
    match op {
        Add(_) => "+",
        Sub(_) => "-",
        Mul(_) => "*",
        Div(_) => "/",
        Rem(_) => "%",
        And(_) => "&&",
        Or(_) => "||",
        BitXor(_) => "^",
        BitAnd(_) => "&",
        BitOr(_) => "|",
        Shl(_) => "<<",
        Shr(_) => ">>",
        Eq(_) => "==",
        Lt(_) => "<",
        Le(_) => "<=",
        Ne(_) => "!=",
        Ge(_) => ">=",
        Gt(_) => ">",
        AddEq(_) => "+=",
        SubEq(_) => "-=",
        MulEq(_) => "*=",
        DivEq(_) => "/=",
        RemEq(_) => "%=",
        BitXorEq(_) => "^=",
        BitAndEq(_) => "&=",
        BitOrEq(_) => "|=",
        ShlEq(_) => "<<=",
        ShrEq(_) => ">>=",
    }
}

// test if two dependencies resolve to the same source
fn test_if_dependency_same_source(a: String, b: String, let_stmts: &HashMap<LetStatement, LetInitialization>)
-> bool
{
    false
}

// resolve a dependency to a range
fn resolve_dependency(
    init: &LetInitialization,
    let_stmts: &HashMap<LetStatement, LetInitialization>,
    func_args: &HashMap<String, Range<usize>>)
-> Range<usize>
{
    use std::cell::UnsafeCell;

    let range_init = UnsafeCell::new(LetInitialization::Range(0..0));
    let mut last_init = init;

    while let LetInitialization::Dependency(d) = last_init {
        if let Some(e) = let_stmts.get(&LetStatement { variable_name: d.clone() }) {
            last_init = e;
        } else if let Some(e) = func_args.get(d) {
            unsafe { *range_init.get() = LetInitialization::Range(e.clone()) };
            last_init = unsafe { &*range_init.get() };
        } else {
            panic!("unresolvable dependency: {}", d);
        }
    }

    last_init.as_range().unwrap()
}

fn check_for_illogical_dependencies() {

}