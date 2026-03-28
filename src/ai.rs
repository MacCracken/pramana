//! Natural language statistical queries via [hoosh](https://docs.rs/hoosh).
//!
//! Requires the `ai` feature flag. Provides [`StatQuery`] which uses an LLM
//! (via hoosh) to interpret natural-language questions about data and dispatch
//! the appropriate pramana operations.

use crate::error::PramanaError;
use crate::{descriptive, hypothesis, regression, timeseries};
use hoosh::inference::{Message, Role};
use hoosh::{HooshClient, InferenceRequest, ToolCall, ToolChoice, ToolDefinition};
use serde_json::{Value, json};
use std::collections::HashMap;

/// A natural-language statistical query engine backed by an LLM.
pub struct StatQuery {
    client: HooshClient,
    model: String,
}

/// Result of a natural-language statistical query.
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Human-readable answer.
    pub answer: String,
    /// Name of the statistical operation performed.
    pub operation: String,
    /// Computed values (e.g. "mean" -> 3.5).
    pub values: HashMap<String, f64>,
}

impl StatQuery {
    /// Creates a new `StatQuery` using the given hoosh client and model name.
    pub fn new(client: HooshClient, model: impl Into<String>) -> Self {
        Self {
            client,
            model: model.into(),
        }
    }

    /// Asks a natural-language question about the provided data.
    ///
    /// `data` maps column names to observation vectors. The LLM chooses which
    /// pramana operations to run and formats the answer.
    ///
    /// # Errors
    ///
    /// Returns `InvalidSample` if `data` is empty.
    /// Returns `ComputationError` if the LLM call or operation fails.
    pub async fn query(
        &self,
        data: &HashMap<String, Vec<f64>>,
        question: &str,
    ) -> Result<QueryResult, PramanaError> {
        if data.is_empty() {
            return Err(PramanaError::InvalidSample("data must be non-empty".into()));
        }

        // Build data summary for the system prompt
        let summary = build_data_summary(data);
        let system = format!(
            "You are a statistical analysis assistant. The user has a dataset with these columns:\n\n\
             {summary}\n\n\
             Use the provided tools to answer the user's question. \
             Call exactly one tool, then explain the result concisely."
        );

        let tools = build_tool_definitions();

        let request = InferenceRequest {
            model: self.model.clone(),
            prompt: question.to_string(),
            system: Some(system),
            tools,
            tool_choice: Some(ToolChoice::Auto),
            ..Default::default()
        };

        let response =
            self.client.infer(&request).await.map_err(|e| {
                PramanaError::ComputationError(format!("LLM inference failed: {e}"))
            })?;

        // If the model called a tool, execute it
        if let Some(tool_call) = response.tool_calls.first() {
            let (operation, values, result_text) = execute_tool(tool_call, data)?;

            // Send the tool result back for a final answer
            let messages = vec![
                Message::new(Role::User, question),
                Message::new(Role::Tool, &result_text),
            ];

            let followup = InferenceRequest {
                model: self.model.clone(),
                prompt: format!(
                    "The tool `{}` returned: {}\n\nSummarize this for the user in one or two sentences.",
                    tool_call.name, result_text
                ),
                messages,
                ..Default::default()
            };

            let answer = self
                .client
                .infer(&followup)
                .await
                .map(|r| r.text)
                .unwrap_or(result_text.clone());

            Ok(QueryResult {
                answer,
                operation,
                values,
            })
        } else {
            // No tool call — model answered directly
            Ok(QueryResult {
                answer: response.text,
                operation: "direct_answer".into(),
                values: HashMap::new(),
            })
        }
    }
}

/// Builds a text summary of the dataset for the LLM context.
fn build_data_summary(data: &HashMap<String, Vec<f64>>) -> String {
    let mut lines = Vec::new();
    for (name, values) in data {
        let n = values.len();
        if n == 0 {
            lines.push(format!("- `{name}`: empty"));
            continue;
        }
        let mean = descriptive::mean(values).unwrap_or(f64::NAN);
        let sd = descriptive::std_dev(values).unwrap_or(f64::NAN);
        let mn = descriptive::min(values).unwrap_or(f64::NAN);
        let mx = descriptive::max(values).unwrap_or(f64::NAN);
        lines.push(format!(
            "- `{name}`: {n} observations, mean={mean:.4}, std={sd:.4}, min={mn:.4}, max={mx:.4}"
        ));
    }
    lines.join("\n")
}

/// Defines the tools the LLM can call.
fn build_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "describe".into(),
            description: "Compute descriptive statistics (mean, median, std_dev, variance, min, max, skewness, kurtosis) for a column.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "column": { "type": "string", "description": "Column name" }
                },
                "required": ["column"]
            }),
        },
        ToolDefinition {
            name: "correlate".into(),
            description: "Compute the Pearson correlation between two columns.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "column_a": { "type": "string", "description": "First column" },
                    "column_b": { "type": "string", "description": "Second column" }
                },
                "required": ["column_a", "column_b"]
            }),
        },
        ToolDefinition {
            name: "regress".into(),
            description: "Fit a linear regression y = slope*x + intercept.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "x_column": { "type": "string", "description": "Independent variable column" },
                    "y_column": { "type": "string", "description": "Dependent variable column" }
                },
                "required": ["x_column", "y_column"]
            }),
        },
        ToolDefinition {
            name: "t_test".into(),
            description: "Run a two-sample t-test comparing two columns, or a one-sample t-test against a given mean.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "column_a": { "type": "string", "description": "First column (required)" },
                    "column_b": { "type": "string", "description": "Second column (optional, for two-sample test)" },
                    "mu": { "type": "number", "description": "Hypothesized mean (for one-sample test, default 0)" }
                },
                "required": ["column_a"]
            }),
        },
        ToolDefinition {
            name: "forecast".into(),
            description: "Fit an AR model and forecast future values of a time series column.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "column": { "type": "string", "description": "Time series column" },
                    "steps": { "type": "integer", "description": "Number of steps to forecast (default 5)" },
                    "ar_order": { "type": "integer", "description": "AR order p (default 1)" }
                },
                "required": ["column"]
            }),
        },
    ]
}

/// Executes a tool call against the dataset.
fn execute_tool(
    tool_call: &ToolCall,
    data: &HashMap<String, Vec<f64>>,
) -> Result<(String, HashMap<String, f64>, String), PramanaError> {
    let args = &tool_call.arguments;

    fn get_col<'a>(
        data: &'a HashMap<String, Vec<f64>>,
        args: &Value,
        key: &str,
    ) -> Result<&'a [f64], PramanaError> {
        let name = args[key]
            .as_str()
            .ok_or_else(|| PramanaError::InvalidParameter(format!("missing {key}")))?;
        data.get(name)
            .map(|v| v.as_slice())
            .ok_or_else(|| PramanaError::InvalidParameter(format!("column '{name}' not found")))
    }

    match tool_call.name.as_str() {
        "describe" => {
            let col = get_col(data, args, "column")?;
            let name = args["column"].as_str().unwrap_or("?");
            let m = descriptive::mean(col)?;
            let med = descriptive::median(col)?;
            let sd = descriptive::std_dev(col)?;
            let v = descriptive::variance(col)?;
            let mn = descriptive::min(col)?;
            let mx = descriptive::max(col)?;
            let mut values = HashMap::new();
            values.insert("mean".into(), m);
            values.insert("median".into(), med);
            values.insert("std_dev".into(), sd);
            values.insert("variance".into(), v);
            values.insert("min".into(), mn);
            values.insert("max".into(), mx);
            let text = format!(
                "Descriptive stats for '{name}': mean={m:.4}, median={med:.4}, \
                 std_dev={sd:.4}, variance={v:.4}, min={mn:.4}, max={mx:.4}"
            );
            Ok(("describe".into(), values, text))
        }
        "correlate" => {
            let a = get_col(data, args, "column_a")?;
            let b = get_col(data, args, "column_b")?;
            let matrix = descriptive::correlation_matrix(&[a, b])?;
            let r = matrix[0][1];
            let mut values = HashMap::new();
            values.insert("correlation".into(), r);
            let na = args["column_a"].as_str().unwrap_or("a");
            let nb = args["column_b"].as_str().unwrap_or("b");
            let text = format!("Pearson correlation between '{na}' and '{nb}': r = {r:.4}");
            Ok(("correlate".into(), values, text))
        }
        "regress" => {
            let x = get_col(data, args, "x_column")?;
            let y = get_col(data, args, "y_column")?;
            let model = regression::linear_regression(x, y)?;
            let mut values = HashMap::new();
            values.insert("slope".into(), model.slope);
            values.insert("intercept".into(), model.intercept);
            values.insert("r_squared".into(), model.r_squared);
            let xn = args["x_column"].as_str().unwrap_or("x");
            let yn = args["y_column"].as_str().unwrap_or("y");
            let text = format!(
                "Linear regression {yn} = {:.4} * {xn} + {:.4} (R² = {:.4})",
                model.slope, model.intercept, model.r_squared
            );
            Ok(("regress".into(), values, text))
        }
        "t_test" => {
            let a = get_col(data, args, "column_a")?;
            if let Some(b_name) = args.get("column_b").and_then(|v| v.as_str()) {
                let b = data.get(b_name).map(|v| v.as_slice()).ok_or_else(|| {
                    PramanaError::InvalidParameter(format!("column '{b_name}' not found"))
                })?;
                let result = hypothesis::t_test_two_sample(a, b, 0.05)?;
                let mut values = HashMap::new();
                values.insert("t_statistic".into(), result.statistic);
                values.insert("p_value".into(), result.p_value);
                values.insert("df".into(), result.degrees_of_freedom);
                let na = args["column_a"].as_str().unwrap_or("a");
                let text = format!(
                    "Two-sample t-test ({na} vs {b_name}): t = {:.4}, p = {:.4}, df = {:.1}, reject = {}",
                    result.statistic, result.p_value, result.degrees_of_freedom, result.reject
                );
                Ok(("t_test_two_sample".into(), values, text))
            } else {
                let mu = args.get("mu").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let result = hypothesis::t_test_one_sample(a, mu, 0.05)?;
                let mut values = HashMap::new();
                values.insert("t_statistic".into(), result.statistic);
                values.insert("p_value".into(), result.p_value);
                values.insert("df".into(), result.degrees_of_freedom);
                let na = args["column_a"].as_str().unwrap_or("a");
                let text = format!(
                    "One-sample t-test ({na} vs mu={mu}): t = {:.4}, p = {:.4}, df = {:.1}, reject = {}",
                    result.statistic, result.p_value, result.degrees_of_freedom, result.reject
                );
                Ok(("t_test_one_sample".into(), values, text))
            }
        }
        "forecast" => {
            let col = get_col(data, args, "column")?;
            let steps = args.get("steps").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
            let p = args.get("ar_order").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
            let model = timeseries::arima_fit(col, p, 0)?;
            let fc = timeseries::arima_forecast(&model, col, steps)?;
            let mut values = HashMap::new();
            for (i, &v) in fc.iter().enumerate() {
                values.insert(format!("forecast_{}", i + 1), v);
            }
            let name = args["column"].as_str().unwrap_or("?");
            let fc_str: Vec<String> = fc.iter().map(|v| format!("{v:.4}")).collect();
            let text = format!(
                "AR({p}) forecast for '{name}', next {steps} values: [{}]",
                fc_str.join(", ")
            );
            Ok(("forecast".into(), values, text))
        }
        other => Err(PramanaError::InvalidParameter(format!(
            "unknown tool: {other}"
        ))),
    }
}
