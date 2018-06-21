extern crate syn;
extern crate quote; // only used for debugging / dumping as text

use syn::synom::ParseError;
use syn::{Item, ItemFn, FnDecl, Block, BinOp, parse_file};
use std::collections::HashMap;
use std::ops::Range;

mod range_comp;

const TEST_STRING: &str = include_str!("./test.rs");

fn main() {
    // step 1: list the functions in the file and return an AST for each function
    let functions = break_file_into_functions(TEST_STRING).unwrap();
    let functions: Vec<FunctionVariables> = functions.into_iter().map(|func| list_variables_in_function(func)).collect();
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

#[derive(Debug, Clone)]
struct FunctionVariables {
    /// Name of this function
    pub name: String,
    // i.e. let x = 5;
    //
    // translates to an entry:
    // ("x", Range { start: 5, end: 5 })
    pub known_variable_items: HashMap<String, Range<usize>>,
    pub dependencies: HashMap<String, Vec<Dependency>>,
}

#[derive(Debug, Clone)]
struct Dependency {
    // What variable this depends on
    dependent_on: String,
}

fn list_variables_in_function(func: ItemFn) -> FunctionVariables {

    let function_name = func.ident.to_string();
    let variable_sources = parse_function_arguments(&func.decl);

    let internal_let_statements = parse_let_statements(&func.block);

    let if_conditions = parse_if_conditions(&func.block);
    println!("function: {} --------", function_name);
    println!("external sources: {:?}", variable_sources);
    println!("let statements: {:?}", internal_let_statements);
    println!("if conditions: {:?}", if_conditions);
    println!("----------", );

    FunctionVariables {
        name: function_name,
        known_variable_items: HashMap::default(),
        dependencies: HashMap::default(),
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct LetStatement {
    variable_name: String,
    // position: Span,
}

/// `let a = 5;` -> ["a @ line 1; (5..5)"]
fn parse_let_statements(block: &Block) -> HashMap<LetStatement, Range<usize>> {
    use syn::{Stmt, Local, token::Let, Pat, Lit, Expr, ExprLit};

    let mut let_statements = HashMap::default();

    for stmt in &block.stmts {
        if let Stmt::Local(Local { let_token: Let(_), pats, init, .. }) = stmt {
            let local_variable_name = match pats.iter().last() {
                Some(Pat::Ident(t)) => t.ident.to_string(),
                _ => continue,
            };
            // Lit(ExprLit { attrs: [], lit: Int(LitInt { token: Literal { lit: 5 }
            let initial_value = if let Some(Expr::Lit(ExprLit { lit: Lit::Int(l), .. })) = init.as_ref().and_then(|i| Some(&*i.1)) {
                let l = l.value() as usize; // warn: cast
                l..l
            } else {
                continue;
            };
            let_statements.insert(LetStatement { variable_name: local_variable_name }, initial_value);
        }
    }

    let_statements
}

#[derive(Debug, Clone)]
struct IfExpression {
    var1: String,
    cond: BinOp,
    var2: String,
    // position: Span,
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

/// Parses possible sources of variables passed into a function via inputs:
///
/// `fn(a: usize, b: usize)` -> `["a", (USIZE_MIN..USIZE_MAX), "b", (USIZE_MIN..USIZE_MAX)]`
fn parse_function_arguments(function_args: &FnDecl)
-> HashMap<String, Range<usize>>
{
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