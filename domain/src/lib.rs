pub mod models;
pub mod repositories;

use std::{borrow::Cow, str::FromStr};

use enum_display::EnumDisplay;
use serde::Deserialize;
use time::Date;
use utils::time::DATE_FORMAT;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DomainErrorKind {
    /// 検証エラー
    Validation,
    /// エンティティが存在しない
    NotFound,
    /// 認証されていない
    Unauthorized,
    /// 禁止された操作
    Forbidden,
    /// リポジトリエラー
    Repository,
    /// 予期しないエラー
    Unexpected,
}

impl std::fmt::Display for DomainErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomainErrorKind::Validation => write!(f, "Validation Error"),
            DomainErrorKind::NotFound => write!(f, "Not Found"),
            DomainErrorKind::Unauthorized => write!(f, "Unauthorized"),
            DomainErrorKind::Forbidden => write!(f, "Forbidden"),
            DomainErrorKind::Repository => write!(f, "Repository Error"),
            DomainErrorKind::Unexpected => write!(f, "Unexpected Error"),
        }
    }
}

/// ドメインエラー
#[derive(Debug, thiserror::Error)]
pub struct DomainError {
    pub kind: DomainErrorKind,
    pub messages: Vec<Cow<'static, str>>,
    pub source: anyhow::Error,
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DomainError: {} - {:?}", self.kind, self.messages)
    }
}

pub fn domain_error(kind: DomainErrorKind, message: &'static str) -> DomainError {
    DomainError {
        kind,
        messages: vec![message.into()],
        source: anyhow::anyhow!(message),
    }
}

/// ドメイン結果
pub type DomainResult<T> = Result<T, DomainError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumDisplay, Deserialize)]
#[enum_display(case = "Snake")]
#[serde(rename_all = "snake_case")]
pub enum NumericOperator {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    Between,
    NotBetween,
    IsNull,
    IsNotNull,
}

impl NumericOperator {
    fn sql(self) -> &'static str {
        match self {
            NumericOperator::Eq => "=",
            NumericOperator::Ne => "<>",
            NumericOperator::Gt => ">",
            NumericOperator::Gte => ">=",
            NumericOperator::Lt => "<",
            NumericOperator::Lte => "<=",
            NumericOperator::Between => "BETWEEN",
            NumericOperator::NotBetween => "NOT BETWEEN",
            NumericOperator::IsNull => "IS NULL",
            NumericOperator::IsNotNull => "IS NOT NULL",
        }
    }
}

impl FromStr for NumericOperator {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "eq" => Ok(NumericOperator::Eq),
            "ne" => Ok(NumericOperator::Ne),
            "gt" => Ok(NumericOperator::Gt),
            "gte" => Ok(NumericOperator::Gte),
            "lt" => Ok(NumericOperator::Lt),
            "lte" => Ok(NumericOperator::Lte),
            "between" => Ok(NumericOperator::Between),
            "not_between" => Ok(NumericOperator::NotBetween),
            _ => Err(format!("Unknown numeric operator: {}", s)),
        }
    }
}

/// 数値フィルター
#[derive(Debug, Clone, Copy)]
pub struct NumericFilter<T>
where
    T: std::fmt::Display + PartialOrd,
{
    pub op: NumericOperator,
    pub from: Option<T>,
    pub to: Option<T>,
}

const NUMERIC_FILTER_MISSING_FROM: &str = "Numeric filter requires 'from' value";
const NUMERIC_FILTER_MISSING_TO: &str = "Numeric filter requires 'to' value";
const NUMERIC_FILTER_TO_LESS_THAN_FROM: &str = "'to' value is less than 'from' value";

pub type DateFilter = NumericFilter<Date>;

impl DateFilter {
    pub fn new(op: NumericOperator, from: Option<Date>, to: Option<Date>) -> DomainResult<Self> {
        if op != NumericOperator::IsNull && op != NumericOperator::IsNotNull && from.is_none() {
            return Err(domain_error(
                DomainErrorKind::Validation,
                NUMERIC_FILTER_MISSING_FROM,
            ));
        }
        if op == NumericOperator::Between || op == NumericOperator::NotBetween {
            if to.is_none() {
                return Err(domain_error(
                    DomainErrorKind::Validation,
                    NUMERIC_FILTER_MISSING_TO,
                ));
            }
            if to.unwrap() < from.unwrap() {
                return Err(domain_error(
                    DomainErrorKind::Validation,
                    NUMERIC_FILTER_TO_LESS_THAN_FROM,
                ));
            }
        }
        Ok(Self { op, from, to })
    }

    pub fn sql(&self, column: &str) -> String {
        match self.op {
            NumericOperator::Eq
            | NumericOperator::Gt
            | NumericOperator::Gte
            | NumericOperator::Lt
            | NumericOperator::Lte => {
                format!(
                    "{column} {} '{}'",
                    self.op.sql(),
                    self.from.unwrap().format(&DATE_FORMAT).unwrap()
                )
            }
            NumericOperator::Ne => {
                format!(
                    "({column} {} '{}' OR {column} IS NULL)",
                    self.op.sql(),
                    self.from.unwrap().format(&DATE_FORMAT).unwrap()
                )
            }
            NumericOperator::Between => {
                format!(
                    "{column} {} '{}' AND '{}'",
                    self.op.sql(),
                    self.from.unwrap().format(&DATE_FORMAT).unwrap(),
                    self.to.unwrap().format(&DATE_FORMAT).unwrap()
                )
            }
            NumericOperator::NotBetween => {
                format!(
                    "({column} {} '{}' AND '{}' OR {column} IS NULL)",
                    self.op.sql(),
                    self.from.unwrap().format(&DATE_FORMAT).unwrap(),
                    self.to.unwrap().format(&DATE_FORMAT).unwrap()
                )
            }
            NumericOperator::IsNull => {
                format!("{column} IS NULL")
            }
            NumericOperator::IsNotNull => {
                format!("{column} IS NOT NULL")
            }
        }
    }
}

/// 文字列が空白で始まるか、または空白で終わるかをチェックする。
pub fn starts_or_ends_with_whitespace(s: &str) -> bool {
    s.chars().next().is_some_and(|ch| ch.is_whitespace())
        || s.chars().last().is_some_and(|ch| ch.is_whitespace())
}

#[cfg(test)]
mod tests {
    use time::macros::date;

    use super::*;

    #[rstest::rstest]
    #[case(" leading space", true)]
    #[case("trailing space ", true)]
    #[case(" both sides ", true)]
    #[case("no spaces", false)]
    #[case("", false)]
    #[case("   ", true)]
    fn starts_or_ends_with_whitespace_ok(#[case] target: &str, #[case] expected: bool) {
        assert_eq!(starts_or_ends_with_whitespace(target), expected);
    }

    #[rstest::rstest]
    #[case(NumericOperator::Eq, "=")]
    #[case(NumericOperator::Ne, "<>")]
    #[case(NumericOperator::Gt, ">")]
    #[case(NumericOperator::Gte, ">=")]
    #[case(NumericOperator::Lt, "<")]
    #[case(NumericOperator::Lte, "<=")]
    #[case(NumericOperator::Between, "BETWEEN")]
    #[case(NumericOperator::NotBetween, "NOT BETWEEN")]
    fn numeric_operator_sql_ok(#[case] op: NumericOperator, #[case] expected: &str) {
        assert_eq!(op.sql(), expected);
    }

    #[rstest::rstest]
    #[case("eq", NumericOperator::Eq)]
    #[case("ne", NumericOperator::Ne)]
    #[case("gt", NumericOperator::Gt)]
    #[case("gte", NumericOperator::Gte)]
    #[case("lt", NumericOperator::Lt)]
    #[case("lte", NumericOperator::Lte)]
    #[case("between", NumericOperator::Between)]
    #[case("not_between", NumericOperator::NotBetween)]
    fn numeric_operator_from_str_ok(#[case] op: &str, #[case] expected: NumericOperator) {
        let actual = NumericOperator::from_str(op).unwrap();
        assert_eq!(actual, expected);
    }

    #[rstest::rstest]
    #[case(NumericOperator::Eq, Some(date!(2025 - 01 - 01)), None, "col", "col = '2025-01-01'")]
    #[case(NumericOperator::Ne, Some(date!(2025 - 01 - 01)), None, "col", "(col <> '2025-01-01' OR col IS NULL)")]
    #[case(NumericOperator::Gt, Some(date!(2025 - 01 - 01)), None, "col", "col > '2025-01-01'")]
    #[case(NumericOperator::Gt, Some(date!(2025 - 01 - 01)), None, "col", "col > '2025-01-01'")]
    #[case(NumericOperator::Lt, Some(date!(2025 - 01 - 01)), None, "col", "col < '2025-01-01'")]
    #[case(NumericOperator::Lte, Some(date!(2025- 01 - 01)), None, "col", "col <= '2025-01-01'")]
    #[case(NumericOperator::Between, Some(date!(2025 - 01 - 01)), Some(date!(2025 - 01 - 01)), "col", "col BETWEEN '2025-01-01' AND '2025-01-01'")]
    #[case(NumericOperator::NotBetween, Some(date!(2025 - 01 - 01)), Some(date!(2025 - 01 - 31)), "col", "(col NOT BETWEEN '2025-01-01' AND '2025-01-31' OR col IS NULL)")]
    #[case(NumericOperator::IsNull, None, None, "col", "col IS NULL")]
    #[case(NumericOperator::IsNotNull, None, None, "col", "col IS NOT NULL")]
    fn date_filter_new_ok(
        #[case] op: NumericOperator,
        #[case] from: Option<Date>,
        #[case] to: Option<Date>,
        #[case] column: &str,
        #[case] expected: &str,
    ) -> anyhow::Result<()> {
        let filter = DateFilter::new(op, from, to)?;
        let actual = filter.sql(column);
        assert_eq!(actual, expected, "expected: {expected}, actual: {actual}");
        Ok(())
    }

    #[rstest::rstest]
    #[case(NumericOperator::Between, Some(date!(2025 - 01 - 01)), None, NUMERIC_FILTER_MISSING_TO)]
    #[case(NumericOperator::NotBetween, Some(date!(2025 - 01 - 01)), None, NUMERIC_FILTER_MISSING_TO)]
    #[case(NumericOperator::Between, Some(date!(2025 - 01 - 01)), Some(date!(2024 - 12 - 31)), NUMERIC_FILTER_TO_LESS_THAN_FROM)]
    fn date_filter_new_err(
        #[case] op: NumericOperator,
        #[case] from: Option<Date>,
        #[case] to: Option<Date>,
        #[case] expected: &str,
    ) {
        let result = DateFilter::new(op, from, to);
        assert!(result.is_err());
        assert!(format!("{}", result.err().unwrap()).contains(expected));
    }
}
