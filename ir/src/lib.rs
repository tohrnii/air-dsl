use std::collections::BTreeMap;

use parser::ast;
pub use parser::ast::{boundary_constraints::BoundaryExpr, Identifier, PublicInput};

mod symbol_table;
use symbol_table::{IdentifierType, SymbolTable};

pub mod boundary_constraints;
use boundary_constraints::BoundaryConstraints;

pub mod transition_constraints;
pub use transition_constraints::NodeIndex;
use transition_constraints::{AlgebraicGraph, TransitionConstraints};

mod error;
use error::SemanticError;

/// Internal representation of an AIR.
///
/// TODO: docs
#[derive(Default, Debug)]
pub struct AirIR {
    air_name: String,
    boundary_constraints: BoundaryConstraints,
    transition_constraints: TransitionConstraints,
}

impl AirIR {
    // --- CONSTRUCTOR ----------------------------------------------------------------------------

    /// Consumes the provided source and generates a matching AirIR.
    pub fn from_source(source: &ast::Source) -> Result<Self, SemanticError> {
        let ast::Source(source) = source;
        // set a default name.
        let mut air_name = "CustomAir";

        // process the declarations of identifiers first, using a single symbol table to enforece
        // uniqueness.
        let mut symbol_table = SymbolTable::default();
        for section in source {
            match section {
                ast::SourceSection::AirDef(Identifier(air_def)) => {
                    air_name = air_def;
                }
                ast::SourceSection::TraceCols(columns) => {
                    // process & validate the main trace columns
                    for (idx, Identifier(name)) in columns.main_cols.iter().enumerate() {
                        symbol_table.insert_main_trace_column(name, idx)?;
                    }
                    // process & validate the auxiliary trace columns
                    for (idx, Identifier(name)) in columns.aux_cols.iter().enumerate() {
                        symbol_table.insert_aux_trace_column(name, idx)?;
                    }
                }
                ast::SourceSection::PublicInputs(inputs) => {
                    // process & validate the public inputs
                    for input in inputs.iter() {
                        symbol_table.insert_public_input(input.name(), input.size())?;
                    }
                }
                ast::SourceSection::PeriodicColumns(columns) => {
                    // process & validate the periodic columns
                    for (idx, column) in columns.iter().enumerate() {
                        symbol_table.insert_periodic_column(column.name(), idx, column.values())?;
                    }
                }
                _ => {}
            }
        }

        // then process the constraints.
        let mut boundary_constraints = BoundaryConstraints::default();
        let mut transition_constraints = TransitionConstraints::default();
        for section in source {
            match section {
                ast::SourceSection::BoundaryConstraints(constraints) => {
                    for constraint in constraints.boundary_constraints.iter() {
                        boundary_constraints.insert(&symbol_table, constraint)?;
                    }
                }
                ast::SourceSection::TransitionConstraints(constraints) => {
                    for constraint in constraints.transition_constraints.iter() {
                        transition_constraints.insert(&symbol_table, constraint)?;
                    }
                }
                _ => {}
            }
        }

        Ok(Self {
            air_name: air_name.to_string(),
            boundary_constraints,
            transition_constraints,
        })
    }

    // --- PUBLIC ACCESSORS -----------------------------------------------------------------------

    pub fn air_name(&self) -> &str {
        &self.air_name
    }

    // --- PUBLIC ACCESSORS FOR BOUNDARY CONSTRAINTS ----------------------------------------------

    pub fn num_main_assertions(&self) -> usize {
        self.boundary_constraints.main_len()
    }

    pub fn main_first_boundary_constraints(&self) -> Vec<&BoundaryExpr> {
        self.boundary_constraints.main_first()
    }

    pub fn main_last_boundary_constraints(&self) -> Vec<&BoundaryExpr> {
        self.boundary_constraints.main_last()
    }

    pub fn num_aux_assertions(&self) -> usize {
        self.boundary_constraints.aux_len()
    }

    pub fn aux_first_boundary_constraints(&self) -> Vec<&BoundaryExpr> {
        self.boundary_constraints.aux_first()
    }

    pub fn aux_last_boundary_constraints(&self) -> Vec<&BoundaryExpr> {
        self.boundary_constraints.aux_last()
    }

    // --- PUBLIC ACCESSORS FOR TRANSITION CONSTRAINTS --------------------------------------------

    pub fn main_degrees(&self) -> Vec<u8> {
        self.transition_constraints.main_degrees()
    }

    pub fn main_transition_constraints(&self) -> &[NodeIndex] {
        self.transition_constraints.main_constraints()
    }

    pub fn aux_degrees(&self) -> Vec<u8> {
        self.transition_constraints.aux_degrees()
    }

    pub fn aux_transition_constraints(&self) -> &[NodeIndex] {
        self.transition_constraints.aux_constraints()
    }

    pub fn transition_graph(&self) -> &AlgebraicGraph {
        self.transition_constraints.graph()
    }
}

// TODO: add checks for the correctness of the AirIR that is built.
#[cfg(test)]
mod tests {
    use super::*;
    use parser::parse;

    #[test]
    fn boundary_constraints() {
        let source = "
        trace_columns:
            main: [clk]
        boundary_constraints:
            enf clk.first = 0
            enf clk.last = 1";

        let parsed = parse(source).expect("Parsing failed");

        let result = AirIR::from_source(&parsed);
        assert!(result.is_ok());
    }

    #[test]
    fn err_bc_column_undeclared() {
        let source = "
        boundary_constraints:
            enf clk.first = 0
            enf clk.last = 1";

        let parsed = parse(source).expect("Parsing failed");

        let result = AirIR::from_source(&parsed);
        assert!(result.is_err());
    }

    #[test]
    fn err_bc_duplicate_first() {
        let source = "
        trace_columns:
            main: [clk]
        boundary_constraints:
            enf clk.first = 0
            enf clk.first = 1";

        let parsed = parse(source).expect("Parsing failed");
        let result = AirIR::from_source(&parsed);

        assert!(result.is_err());
    }

    #[test]
    fn err_bc_duplicate_last() {
        let source = "
        trace_columns:
            main: [clk]
        boundary_constraints:
            enf clk.last = 0
            enf clk.last = 1";

        let parsed = parse(source).expect("Parsing failed");

        assert!(AirIR::from_source(&parsed).is_err());
    }

    #[test]
    fn transition_constraints() {
        let source = "
        trace_columns:
            main: [clk]
        transition_constraints:
            enf clk' = clk + 1";

        let parsed = parse(source).expect("Parsing failed");

        let result = AirIR::from_source(&parsed);
        assert!(result.is_ok());
    }

    #[test]
    fn transition_constraints_using_parens() {
        let source = "
        trace_columns:
            main: [clk]
        transition_constraints:
            enf clk' = (clk + 1)";

        let parsed = parse(source).expect("Parsing failed");

        let result = AirIR::from_source(&parsed);
        assert!(result.is_ok());
    }

    #[test]
    fn err_tc_column_undeclared() {
        let source = "
        transition_constraints:
            enf clk' = clk + 1";

        let parsed = parse(source).expect("Parsing failed");

        let result = AirIR::from_source(&parsed);
        assert!(result.is_err());
    }

    #[test]
    fn op_mul() {
        let source = "
        trace_columns:
            main: [clk]
        transition_constraints:
            enf clk' * clk = 1";
        let parsed = parse(source).expect("Parsing failed");

        let result = AirIR::from_source(&parsed);
        assert!(result.is_ok());
    }

    #[test]
    fn op_exp() {
        let source = "
        trace_columns:
            main: [clk]
        transition_constraints:
            enf clk'^2 - clk = 1";
        let parsed = parse(source).expect("Parsing failed");

        let result = AirIR::from_source(&parsed);
        assert!(result.is_ok());
    }
}
