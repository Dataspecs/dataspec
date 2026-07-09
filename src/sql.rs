use sqlparser::ast::{Cte, Query, SetExpr, Statement, TableFactor, TableWithJoins};
use sqlparser::dialect::Dialect;
use sqlparser::parser::Parser;
use std::collections::HashSet;

#[derive(Debug)]
struct DataspecDialect;

impl Dialect for DataspecDialect {
    fn is_identifier_start(&self, ch: char) -> bool {
        ch.is_ascii_lowercase() || ch.is_ascii_uppercase() || ch == '_'
    }

    fn is_identifier_part(&self, ch: char) -> bool {
        ch.is_ascii_lowercase() || ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_'
    }

    fn is_delimited_identifier_start(&self, ch: char) -> bool {
        ch == '"' || ch == '`'
    }
}

fn extract_table_names(ast: &[Statement]) -> HashSet<String> {
    let mut tables: HashSet<String> = HashSet::new();

    for statement in ast {
        match statement {
            Statement::Query(query) => {
                extract_from_query(query, &mut tables);
            }
            Statement::Insert(insert) => {
                tables.insert(insert.table.to_string());
            }
            Statement::Update { table, .. } => {
                tables.insert(table.to_string());
            }
            Statement::Delete(delete) => {
                for table in &delete.tables {
                    tables.insert(table.to_string());
                }
                match &delete.from {
                    sqlparser::ast::FromTable::WithFromKeyword(table_with_joins) => {
                        for twj in table_with_joins {
                            extract_from_table_with_joins(twj, &mut tables);
                        }
                    }
                    sqlparser::ast::FromTable::WithoutKeyword(table_with_joins) => {
                        for twj in table_with_joins {
                            extract_from_table_with_joins(twj, &mut tables);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    tables
}

fn extract_from_query(query: &Query, tables: &mut HashSet<String>) {
    let mut ctes: HashSet<String> = HashSet::new();

    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            extract_from_cte(cte, tables);
            ctes.insert(cte.alias.name.to_string());
        }
    }

    extract_from_set_expr(&query.body, tables);
    tables.retain(|t| !ctes.contains(t));
}

fn extract_from_cte(cte: &Cte, tables: &mut HashSet<String>) {
    extract_from_query(&cte.query, tables);
}

fn extract_from_set_expr(set_expr: &SetExpr, tables: &mut HashSet<String>) {
    match set_expr {
        SetExpr::Select(select) => {
            for table_with_joins in &select.from {
                extract_from_table_with_joins(table_with_joins, tables);
            }
        }
        SetExpr::Query(query) => {
            extract_from_query(query, tables);
        }
        SetExpr::SetOperation { left, right, .. } => {
            extract_from_set_expr(left, tables);
            extract_from_set_expr(right, tables);
        }
        _ => {}
    }
}

fn extract_from_table_with_joins(table_with_joins: &TableWithJoins, tables: &mut HashSet<String>) {
    extract_from_table_factor(&table_with_joins.relation, tables);

    for join in &table_with_joins.joins {
        extract_from_table_factor(&join.relation, tables);
    }
}

fn extract_from_table_factor(table_factor: &TableFactor, tables: &mut HashSet<String>) {
    match table_factor {
        TableFactor::Table { name, .. } => {
            tables.insert(name.to_string());
        }
        TableFactor::Derived { subquery, .. } => {
            extract_from_query(subquery, tables);
        }
        TableFactor::NestedJoin {
            table_with_joins, ..
        } => {
            extract_from_table_with_joins(table_with_joins, tables);
        }
        _ => {}
    }
}

fn is_mustache_table_ref(tag: &str) -> bool {
    !tag.starts_with("props__")
        && !tag.starts_with("vars__")
        && !tag.starts_with("var__")
        && tag != "session_id"
}

fn extract_mustache_table_refs(sql: &str) -> HashSet<String> {
    let mut tables = HashSet::new();
    for tag in crate::context::render::extract_mustache_tags(sql) {
        if is_mustache_table_ref(&tag) {
            tables.insert(format!("{{{{{tag}}}}}"));
        }
    }
    tables
}

pub fn get_dependent_tables(sql: &str) -> Vec<String> {
    let mut tables = extract_mustache_table_refs(sql);

    match Parser::parse_sql(&DataspecDialect, sql) {
        Ok(ast) => tables.extend(extract_table_names(&ast)),
        Err(e) => {
            tracing::debug!("SQL parse error while extracting dependent tables: {e}");
        }
    }

    let mut tables: Vec<String> = tables.into_iter().collect();
    tables.sort();
    tables
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_tables_from_select_join() {
        let sql = "SELECT a.id FROM foo a JOIN bar b ON a.id = b.foo_id";
        assert_eq!(
            get_dependent_tables(sql),
            vec!["bar".to_string(), "foo".to_string()]
        );
    }

    #[test]
    fn excludes_cte_aliases() {
        let sql = "WITH cte AS (SELECT * FROM source_table) SELECT * FROM cte JOIN other ON true";
        assert_eq!(
            get_dependent_tables(sql),
            vec!["other".to_string(), "source_table".to_string()]
        );
    }

    #[test]
    fn extracts_mustache_model_refs() {
        assert_eq!(
            get_dependent_tables("SELECT * FROM {{dummy_model}}"),
            vec!["{{dummy_model}}".to_string()]
        );
    }

    #[test]
    fn merges_sql_tables_and_mustache_refs_when_parseable() {
        let sql = "SELECT a.id FROM alpha a JOIN beta b ON a.id = b.a_id";
        assert_eq!(
            get_dependent_tables(sql),
            vec!["alpha".to_string(), "beta".to_string()]
        );
    }

    #[test]
    fn mustache_refs_prevent_sql_parse_of_mixed_query() {
        let sql = "SELECT a.id FROM alpha a JOIN {{beta_model}} b ON a.id = b.a_id";
        assert_eq!(
            get_dependent_tables(sql),
            vec!["{{beta_model}}".to_string()]
        );
    }

    #[test]
    fn ignores_props_vars_and_session_id_mustache_tags() {
        assert_eq!(
            get_dependent_tables("SELECT * FROM {{model_a}} JOIN {{model_b}}"),
            vec!["{{model_a}}".to_string(), "{{model_b}}".to_string()]
        );
        assert_eq!(
            get_dependent_tables(
                "SELECT {{session_id}}, {{vars__year}} FROM t WHERE f = {{props__vata}}"
            ),
            Vec::<String>::new()
        );
    }
}
