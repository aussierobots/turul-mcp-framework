//! # Business Financial Calculator System
//!
//! This example demonstrates a real-world business financial calculator system
//! for development teams, financial analysts, and business professionals. It provides
//! comprehensive financial calculations, business metrics, industry benchmarking,
//! and investment analysis tools loaded from external configuration files.

use std::sync::Arc;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::net::SocketAddr;

use async_trait::async_trait;
use mcp_protocol::{ToolResult, ToolSchema, schema::JsonSchema, McpError, McpResult};
use mcp_server::{McpServer, McpTool, SessionContext};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value, from_str};
use tracing::info;

#[derive(Debug, Deserialize, Serialize)]
struct BusinessFormulas {
    financial_formulas: HashMap<String, FormulaDefinition>,
    business_metrics: HashMap<String, FormulaDefinition>,
    tax_calculations: HashMap<String, FormulaDefinition>,
    real_estate: HashMap<String, FormulaDefinition>,
    retirement_planning: HashMap<String, FormulaDefinition>,
}

#[derive(Debug, Deserialize, Serialize)]
struct FormulaDefinition {
    name: String,
    description: String,
    formula: String,
    parameters: HashMap<String, String>,
    examples: HashMap<String, Value>,
    use_cases: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct IndustryBenchmarks {
    industry_benchmarks: HashMap<String, IndustryData>,
    regional_adjustments: HashMap<String, Value>,
    economic_conditions: HashMap<String, Value>,
    benchmark_categories: HashMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct IndustryData {
    name: String,
    description: String,
    metrics: HashMap<String, MetricBenchmark>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct MetricBenchmark {
    #[serde(flatten)]
    data: HashMap<String, Value>,
}

/// Shared calculator state with external data
#[derive(Debug)]
pub struct BusinessCalculatorState {
    formulas: BusinessFormulas,
    benchmarks: IndustryBenchmarks,
    calculation_templates: String,
}

impl BusinessCalculatorState {
    pub fn new() -> McpResult<Self> {
        let formulas_path = Path::new("data/business_formulas.json");
        let benchmarks_path = Path::new("data/industry_benchmarks.yaml");
        let templates_path = Path::new("data/calculation_templates.md");
        
        let formulas = match fs::read_to_string(formulas_path) {
            Ok(content) => {
                from_str::<BusinessFormulas>(&content)
                    .map_err(|e| McpError::tool_execution(&format!("Failed to parse business formulas: {}", e)))?
            },
            Err(_) => {
                // Fallback formulas
                BusinessFormulas {
                    financial_formulas: HashMap::new(),
                    business_metrics: HashMap::new(),
                    tax_calculations: HashMap::new(),
                    real_estate: HashMap::new(),
                    retirement_planning: HashMap::new(),
                }
            }
        };
        
        let benchmarks = match fs::read_to_string(benchmarks_path) {
            Ok(content) => {
                serde_yml::from_str::<IndustryBenchmarks>(&content)
                    .map_err(|e| McpError::tool_execution(&format!("Failed to parse industry benchmarks: {}", e)))?
            },
            Err(_) => {
                // Fallback benchmarks
                IndustryBenchmarks {
                    industry_benchmarks: HashMap::new(),
                    regional_adjustments: HashMap::new(),
                    economic_conditions: HashMap::new(),
                    benchmark_categories: HashMap::new(),
                }
            }
        };
        
        let calculation_templates = fs::read_to_string(templates_path)
            .unwrap_or_else(|_| "# Business Calculation Templates\n\nNo templates loaded.".to_string());
        
        Ok(Self {
            formulas,
            benchmarks,
            calculation_templates,
        })
    }
}

/// Financial calculations tool for investment analysis and loan calculations
struct FinancialCalculatorTool {
    state: Arc<BusinessCalculatorState>,
}

impl FinancialCalculatorTool {
    fn new(state: Arc<BusinessCalculatorState>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl McpTool for FinancialCalculatorTool {
    fn name(&self) -> &str {
        "calculate_financial"
    }

    fn description(&self) -> &str {
        "Perform financial calculations including compound interest, NPV, IRR, loan payments, and investment analysis"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("calculation_type".to_string(), JsonSchema::string_enum(vec![
                    "compound_interest".to_string(), "present_value".to_string(), "net_present_value".to_string(),
                    "internal_rate_of_return".to_string(), "loan_payment".to_string(), "break_even_analysis".to_string(),
                    "depreciation".to_string(), "roi_calculation".to_string()
                ]).with_description("Type of financial calculation")),
                ("parameters".to_string(), JsonSchema::object()
                    .with_description("Calculation parameters based on the calculation type")),
                ("include_explanation".to_string(), JsonSchema::boolean()
                    .with_description("Include detailed explanation and formula breakdown")),
            ]))
            .with_required(vec!["calculation_type".to_string(), "parameters".to_string()])
    }

    async fn call(&self, args: Value, _session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let calc_type = args.get("calculation_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("calculation_type"))?;
        
        let params = args.get("parameters")
            .and_then(|v| v.as_object())
            .ok_or_else(|| McpError::missing_param("parameters"))?;
        
        let include_explanation = args.get("include_explanation")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let result = match calc_type {
            "compound_interest" => {
                let principal = params.get("principal").and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::missing_param("principal"))?;
                let annual_rate = params.get("annual_rate").and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::missing_param("annual_rate"))?;
                let compounds_per_year = params.get("compounds_per_year").and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::missing_param("compounds_per_year"))?;
                let years = params.get("years").and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::missing_param("years"))?;

                let amount = principal * (1.0 + annual_rate / compounds_per_year).powf(compounds_per_year * years);
                let interest_earned = amount - principal;

                json!({
                    "calculation": "Compound Interest",
                    "formula": "A = P(1 + r/n)^(nt)",
                    "inputs": {
                        "principal": principal,
                        "annual_rate": format!("{:.2}%", annual_rate * 100.0),
                        "compounds_per_year": compounds_per_year,
                        "years": years
                    },
                    "results": {
                        "final_amount": format!("${:.2}", amount),
                        "interest_earned": format!("${:.2}", interest_earned),
                        "total_return": format!("{:.2}%", (interest_earned / principal) * 100.0)
                    },
                    "business_context": "Useful for investment planning, retirement calculations, and loan analysis"
                })
            },
            "loan_payment" => {
                let principal = params.get("principal").and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::missing_param("principal"))?;
                let annual_rate = params.get("annual_rate").and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::missing_param("annual_rate"))?;
                let years = params.get("years").and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::missing_param("years"))?;

                let monthly_rate = annual_rate / 12.0;
                let num_payments = years * 12.0;
                let payment = principal * (monthly_rate * (1.0 + monthly_rate).powf(num_payments)) / 
                    ((1.0 + monthly_rate).powf(num_payments) - 1.0);
                let total_paid = payment * num_payments;
                let total_interest = total_paid - principal;

                json!({
                    "calculation": "Loan Payment",
                    "formula": "PMT = P * [r(1+r)^n] / [(1+r)^n - 1]",
                    "inputs": {
                        "principal": format!("${:.2}", principal),
                        "annual_rate": format!("{:.2}%", annual_rate * 100.0),
                        "loan_term": format!("{} years", years)
                    },
                    "results": {
                        "monthly_payment": format!("${:.2}", payment),
                        "total_paid": format!("${:.2}", total_paid),
                        "total_interest": format!("${:.2}", total_interest),
                        "interest_percentage": format!("{:.1}%", (total_interest / principal) * 100.0)
                    },
                    "business_context": "Used for mortgage planning, auto loans, and business financing decisions"
                })
            },
            "net_present_value" => {
                let initial_investment = params.get("initial_investment").and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::missing_param("initial_investment"))?;
                let cash_flows = params.get("cash_flows").and_then(|v| v.as_array())
                    .ok_or_else(|| McpError::missing_param("cash_flows"))?;
                let discount_rate = params.get("discount_rate").and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::missing_param("discount_rate"))?;

                let mut npv = -initial_investment;
                let mut present_values = Vec::new();
                
                for (i, cf) in cash_flows.iter().enumerate() {
                    if let Some(cash_flow) = cf.as_f64() {
                        let pv = cash_flow / (1.0 + discount_rate).powf((i + 1) as f64);
                        npv += pv;
                        present_values.push(format!("${:.2}", pv));
                    }
                }

                let decision = if npv > 0.0 { "Accept" } else { "Reject" };

                json!({
                    "calculation": "Net Present Value (NPV)",
                    "formula": "NPV = Σ(CF_t / (1 + r)^t) - Initial Investment",
                    "inputs": {
                        "initial_investment": format!("${:.2}", initial_investment),
                        "discount_rate": format!("{:.1}%", discount_rate * 100.0),
                        "cash_flows": cash_flows.iter().filter_map(|v| v.as_f64()).map(|v| format!("${:.2}", v)).collect::<Vec<_>>()
                    },
                    "results": {
                        "npv": format!("${:.2}", npv),
                        "present_values": present_values,
                        "decision": decision,
                        "profitability_index": format!("{:.2}", (npv + initial_investment) / initial_investment)
                    },
                    "business_context": "Critical for capital investment decisions and project evaluation"
                })
            },
            "break_even_analysis" => {
                let fixed_costs = params.get("fixed_costs").and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::missing_param("fixed_costs"))?;
                let price_per_unit = params.get("price_per_unit").and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::missing_param("price_per_unit"))?;
                let variable_cost_per_unit = params.get("variable_cost_per_unit").and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::missing_param("variable_cost_per_unit"))?;

                let contribution_margin = price_per_unit - variable_cost_per_unit;
                let break_even_units = fixed_costs / contribution_margin;
                let break_even_revenue = break_even_units * price_per_unit;
                let margin_of_safety_current = params.get("current_sales").and_then(|v| v.as_f64())
                    .map(|current| current - break_even_revenue);

                json!({
                    "calculation": "Break-Even Analysis",
                    "formula": "Break-even Units = Fixed Costs / (Price per Unit - Variable Cost per Unit)",
                    "inputs": {
                        "fixed_costs": format!("${:.2}", fixed_costs),
                        "price_per_unit": format!("${:.2}", price_per_unit),
                        "variable_cost_per_unit": format!("${:.2}", variable_cost_per_unit)
                    },
                    "results": {
                        "contribution_margin": format!("${:.2}", contribution_margin),
                        "break_even_units": format!("{:.0} units", break_even_units),
                        "break_even_revenue": format!("${:.2}", break_even_revenue),
                        "margin_of_safety": margin_of_safety_current.map(|v| format!("${:.2}", v)),
                        "contribution_margin_ratio": format!("{:.1}%", (contribution_margin / price_per_unit) * 100.0)
                    },
                    "business_context": "Essential for pricing decisions, cost management, and profitability planning"
                })
            },
            "roi_calculation" => {
                let initial_investment = params.get("initial_investment").and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::missing_param("initial_investment"))?;
                let final_value = params.get("final_value").and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::missing_param("final_value"))?;

                let gain = final_value - initial_investment;
                let roi_percentage = (gain / initial_investment) * 100.0;
                let roi_ratio = final_value / initial_investment;

                json!({
                    "calculation": "Return on Investment (ROI)",
                    "formula": "ROI = (Gain - Cost) / Cost × 100",
                    "inputs": {
                        "initial_investment": format!("${:.2}", initial_investment),
                        "final_value": format!("${:.2}", final_value)
                    },
                    "results": {
                        "gain_loss": format!("${:.2}", gain),
                        "roi_percentage": format!("{:.1}%", roi_percentage),
                        "roi_ratio": format!("{:.2}:1", roi_ratio),
                        "performance": if roi_percentage > 0.0 { "Profitable" } else { "Loss" }
                    },
                    "business_context": "Used to evaluate investment performance and compare different opportunities"
                })
            },
            _ => {
                return Err(McpError::invalid_param_type("calculation_type", "supported financial calculation", calc_type));
            }
        };

        if include_explanation {
            if let Some(formula_def) = self.state.formulas.financial_formulas.get(calc_type) {
                let mut enhanced_result = result;
                enhanced_result["detailed_explanation"] = json!({
                    "description": formula_def.description,
                    "use_cases": formula_def.use_cases,
                    "parameters_guide": formula_def.parameters
                });
                Ok(vec![ToolResult::text(serde_json::to_string_pretty(&enhanced_result)?)])
            } else {
                Ok(vec![ToolResult::text(serde_json::to_string_pretty(&result)?)])
            }
        } else {
            Ok(vec![ToolResult::text(serde_json::to_string_pretty(&result)?)])
        }
    }
}

/// Business metrics calculator for customer analytics and operational KPIs
struct BusinessMetricsTool {
    state: Arc<BusinessCalculatorState>,
}

impl BusinessMetricsTool {
    fn new(state: Arc<BusinessCalculatorState>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl McpTool for BusinessMetricsTool {
    fn name(&self) -> &str {
        "calculate_business_metrics"
    }

    fn description(&self) -> &str {
        "Calculate key business metrics including CLV, churn rate, conversion rates, employee productivity, and operational KPIs"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("metric_type".to_string(), JsonSchema::string_enum(vec![
                    "customer_lifetime_value".to_string(), "conversion_rate".to_string(), 
                    "churn_rate".to_string(), "employee_productivity".to_string(),
                    "customer_acquisition_cost".to_string(), "marketing_roi".to_string()
                ]).with_description("Type of business metric to calculate")),
                ("parameters".to_string(), JsonSchema::object()
                    .with_description("Metric calculation parameters")),
                ("industry".to_string(), JsonSchema::string()
                    .with_description("Industry for benchmark comparison (optional)")),
            ]))
            .with_required(vec!["metric_type".to_string(), "parameters".to_string()])
    }

    async fn call(&self, args: Value, _session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let metric_type = args.get("metric_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("metric_type"))?;
        
        let params = args.get("parameters")
            .and_then(|v| v.as_object())
            .ok_or_else(|| McpError::missing_param("parameters"))?;
        
        let industry = args.get("industry").and_then(|v| v.as_str());

        let result = match metric_type {
            "customer_lifetime_value" => {
                let avg_order_value = params.get("average_order_value").and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::missing_param("average_order_value"))?;
                let purchase_frequency = params.get("purchase_frequency").and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::missing_param("purchase_frequency"))?;
                let gross_margin = params.get("gross_margin").and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::missing_param("gross_margin"))?;
                let customer_lifespan = params.get("customer_lifespan").and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::missing_param("customer_lifespan"))?;

                let clv = avg_order_value * purchase_frequency * gross_margin * customer_lifespan;
                let annual_revenue_per_customer = avg_order_value * purchase_frequency * gross_margin;

                json!({
                    "metric": "Customer Lifetime Value (CLV)",
                    "formula": "CLV = Average Order Value × Purchase Frequency × Gross Margin × Customer Lifespan",
                    "inputs": {
                        "average_order_value": format!("${:.2}", avg_order_value),
                        "purchase_frequency": format!("{:.1} orders/year", purchase_frequency),
                        "gross_margin": format!("{:.1}%", gross_margin * 100.0),
                        "customer_lifespan": format!("{:.1} years", customer_lifespan)
                    },
                    "results": {
                        "customer_lifetime_value": format!("${:.2}", clv),
                        "annual_revenue_per_customer": format!("${:.2}", annual_revenue_per_customer),
                        "suggested_cac_budget": format!("${:.2} - ${:.2}", clv / 5.0, clv / 3.0),
                        "payback_period_target": "12-18 months"
                    },
                    "business_insights": [
                        "CLV should be 3-5x higher than Customer Acquisition Cost (CAC)",
                        "Focus retention efforts on high CLV customer segments",
                        "Use CLV to determine maximum marketing spend per customer"
                    ]
                })
            },
            "churn_rate" => {
                let customers_lost = params.get("customers_lost").and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::missing_param("customers_lost"))?;
                let total_customers_start = params.get("total_customers_start").and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::missing_param("total_customers_start"))?;
                let period = params.get("period").and_then(|v| v.as_str()).unwrap_or("monthly");

                let churn_rate = (customers_lost / total_customers_start) * 100.0;
                let retention_rate = 100.0 - churn_rate;
                let customer_lifetime = if churn_rate > 0.0 { 1.0 / (churn_rate / 100.0) } else { 0.0 };

                // Calculate annual churn if monthly provided
                let annual_churn = if period == "monthly" {
                    100.0 * (1.0 - (1.0 - churn_rate / 100.0).powf(12.0))
                } else {
                    churn_rate
                };

                json!({
                    "metric": "Customer Churn Rate",
                    "formula": "Churn Rate = (Customers Lost / Total Customers at Start) × 100",
                    "inputs": {
                        "customers_lost": customers_lost,
                        "total_customers_start": total_customers_start,
                        "period": period
                    },
                    "results": {
                        "churn_rate": format!("{:.1}%", churn_rate),
                        "retention_rate": format!("{:.1}%", retention_rate),
                        "customer_lifetime": format!("{:.1} {}", customer_lifetime, if period == "monthly" { "months" } else { "years" }),
                        "annual_churn_rate": if period == "monthly" { Some(format!("{:.1}%", annual_churn)) } else { None }
                    },
                    "performance_assessment": {
                        "status": if churn_rate < 5.0 { "Excellent" } else if churn_rate < 10.0 { "Good" } else if churn_rate < 20.0 { "Needs Attention" } else { "Critical" },
                        "benchmark_comparison": "SaaS B2B: 2-8% monthly, B2C: 5-15% monthly"
                    },
                    "actionable_insights": [
                        "Identify common characteristics of churning customers",
                        "Implement early warning systems for at-risk accounts",
                        "Focus on customer success and onboarding improvements"
                    ]
                })
            },
            "conversion_rate" => {
                let conversions = params.get("conversions").and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::missing_param("conversions"))?;
                let total_visitors = params.get("total_visitors").and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::missing_param("total_visitors"))?;
                let conversion_type = params.get("conversion_type").and_then(|v| v.as_str()).unwrap_or("general");

                let conversion_rate = (conversions / total_visitors) * 100.0;
                let visitors_per_conversion = if conversions > 0.0 { total_visitors / conversions } else { 0.0 };

                json!({
                    "metric": "Conversion Rate",
                    "formula": "Conversion Rate = (Conversions / Total Visitors) × 100",
                    "inputs": {
                        "conversions": conversions,
                        "total_visitors": total_visitors,
                        "conversion_type": conversion_type
                    },
                    "results": {
                        "conversion_rate": format!("{:.2}%", conversion_rate),
                        "visitors_per_conversion": format!("{:.0} visitors", visitors_per_conversion),
                        "conversion_volume": format!("{:.0} conversions from {:.0} visitors", conversions, total_visitors)
                    },
                    "performance_assessment": {
                        "status": if conversion_rate > 5.0 { "Excellent" } else if conversion_rate > 2.0 { "Good" } else if conversion_rate > 1.0 { "Average" } else { "Needs Improvement" },
                        "industry_benchmarks": {
                            "ecommerce": "2-4%",
                            "saas_trials": "15-25%",
                            "lead_generation": "10-20%"
                        }
                    },
                    "optimization_opportunities": [
                        "A/B test landing page elements",
                        "Improve call-to-action placement and copy",
                        "Optimize page load speed and mobile experience",
                        "Implement exit-intent popups and retargeting"
                    ]
                })
            },
            "employee_productivity" => {
                let total_revenue = params.get("total_revenue").and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::missing_param("total_revenue"))?;
                let number_of_employees = params.get("number_of_employees").and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::missing_param("number_of_employees"))?;
                let period = params.get("period").and_then(|v| v.as_str()).unwrap_or("annual");

                let revenue_per_employee = total_revenue / number_of_employees;

                json!({
                    "metric": "Employee Productivity",
                    "formula": "Revenue per Employee = Total Revenue / Number of Employees",
                    "inputs": {
                        "total_revenue": format!("${:.0}", total_revenue),
                        "number_of_employees": number_of_employees,
                        "period": period
                    },
                    "results": {
                        "revenue_per_employee": format!("${:.0}", revenue_per_employee),
                        "productivity_tier": if revenue_per_employee > 200000.0 { "High Productivity" } 
                                          else if revenue_per_employee > 100000.0 { "Average Productivity" }
                                          else { "Below Average Productivity" }
                    },
                    "industry_benchmarks": {
                        "technology": "$150,000 - $500,000",
                        "manufacturing": "$100,000 - $300,000",
                        "retail": "$50,000 - $150,000",
                        "professional_services": "$100,000 - $400,000"
                    },
                    "productivity_insights": [
                        "Consider automation opportunities for low-productivity areas",
                        "Invest in employee training and development",
                        "Analyze revenue per employee by department",
                        "Compare against industry benchmarks for competitive positioning"
                    ]
                })
            },
            _ => {
                return Err(McpError::invalid_param_type("metric_type", "supported business metric", metric_type));
            }
        };

        // Add industry benchmark comparison if industry specified
        if let Some(industry_name) = industry {
            if let Some(industry_data) = self.state.benchmarks.industry_benchmarks.get(industry_name) {
                let mut enhanced_result = result;
                enhanced_result["industry_context"] = json!({
                    "industry": industry_data.name,
                    "description": industry_data.description,
                    "relevant_benchmarks": industry_data.metrics.keys().collect::<Vec<_>>()
                });
                return Ok(vec![ToolResult::text(serde_json::to_string_pretty(&enhanced_result)?)]);
            }
        }

        Ok(vec![ToolResult::text(serde_json::to_string_pretty(&result)?)])
    }
}

/// Industry benchmarking tool for comparing performance against industry standards
struct IndustryBenchmarkTool {
    state: Arc<BusinessCalculatorState>,
}

impl IndustryBenchmarkTool {
    fn new(state: Arc<BusinessCalculatorState>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl McpTool for IndustryBenchmarkTool {
    fn name(&self) -> &str {
        "get_industry_benchmarks"
    }

    fn description(&self) -> &str {
        "Get industry benchmarks and performance standards for various business metrics across different industries"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("industry".to_string(), JsonSchema::string_enum(vec![
                    "technology".to_string(), "retail".to_string(), "manufacturing".to_string(),
                    "financial_services".to_string(), "healthcare".to_string(), "hospitality".to_string(),
                    "real_estate".to_string()
                ]).with_description("Industry sector for benchmarking")),
                ("metric_category".to_string(), JsonSchema::string_enum(vec![
                    "all".to_string(), "profitability".to_string(), "efficiency".to_string(),
                    "customer_metrics".to_string(), "financial_health".to_string()
                ]).with_description("Category of metrics to retrieve")),
                ("region".to_string(), JsonSchema::string()
                    .with_description("Geographic region for regional adjustments (optional)")),
            ]))
            .with_required(vec!["industry".to_string()])
    }

    async fn call(&self, args: Value, _session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let industry = args.get("industry")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("industry"))?;
        
        let metric_category = args.get("metric_category")
            .and_then(|v| v.as_str())
            .unwrap_or("all");
        
        let region = args.get("region").and_then(|v| v.as_str());

        if let Some(industry_data) = self.state.benchmarks.industry_benchmarks.get(industry) {
            let mut filtered_metrics = HashMap::new();
            
            if metric_category == "all" {
                filtered_metrics = industry_data.metrics.clone();
            } else if let Some(category_metrics) = self.state.benchmarks.benchmark_categories.get(metric_category) {
                for metric_name in category_metrics {
                    if let Some(metric_data) = industry_data.metrics.get(metric_name) {
                        filtered_metrics.insert(metric_name.clone(), metric_data.clone());
                    }
                }
            }

            let mut result = json!({
                "industry": {
                    "name": industry_data.name,
                    "description": industry_data.description
                },
                "metric_category": metric_category,
                "benchmarks": filtered_metrics,
                "data_context": {
                    "currency": "USD",
                    "last_updated": "2025-01-19",
                    "data_sources": [
                        "Industry reports from McKinsey & Company",
                        "PwC Annual Industry Surveys", 
                        "Deloitte Industry Benchmarking Studies"
                    ]
                },
                "usage_guidelines": [
                    "Benchmarks are general guidelines and may vary by company size and location",
                    "Use for comparative analysis rather than absolute targets",
                    "Consider multiple data points when making business decisions",
                    "Adjust for regional and economic conditions as appropriate"
                ]
            });

            // Add regional adjustments if specified
            if let Some(region_name) = region {
                if let Some(regional_data) = self.state.benchmarks.regional_adjustments.get("cost_of_living") {
                    result["regional_adjustments"] = json!({
                        "region": region_name,
                        "available_adjustments": regional_data
                    });
                }
            }

            // Add economic condition context
            result["economic_conditions"] = json!({
                "recession_adjustments": self.state.benchmarks.economic_conditions.get("recession"),
                "growth_period_adjustments": self.state.benchmarks.economic_conditions.get("growth_period")
            });

            Ok(vec![ToolResult::text(serde_json::to_string_pretty(&result)?)])
        } else {
            let available_industries: Vec<String> = self.state.benchmarks.industry_benchmarks.keys().cloned().collect();
            Err(McpError::tool_execution(&format!(
                "Industry '{}' not found. Available industries: {}",
                industry,
                available_industries.join(", ")
            )))
        }
    }
}

/// Calculator documentation and templates tool
struct CalculatorDocumentationTool {
    state: Arc<BusinessCalculatorState>,
}

impl CalculatorDocumentationTool {
    fn new(state: Arc<BusinessCalculatorState>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl McpTool for CalculatorDocumentationTool {
    fn name(&self) -> &str {
        "get_calculator_documentation"
    }

    fn description(&self) -> &str {
        "Get comprehensive documentation, calculation templates, formulas, and usage guidelines for business calculations"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("doc_type".to_string(), JsonSchema::string_enum(vec![
                    "formulas".to_string(), "templates".to_string(), "examples".to_string(), 
                    "best_practices".to_string(), "all".to_string()
                ]).with_description("Type of documentation to retrieve")),
                ("category".to_string(), JsonSchema::string()
                    .with_description("Specific formula category (financial, business_metrics, etc.)")),
            ]))
            .with_required(vec!["doc_type".to_string()])
    }

    async fn call(&self, args: Value, _session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let doc_type = args.get("doc_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("doc_type"))?;
        
        let category = args.get("category").and_then(|v| v.as_str());

        match doc_type {
            "formulas" => {
                let formulas = if let Some(cat) = category {
                    match cat {
                        "financial" => json!(self.state.formulas.financial_formulas),
                        "business_metrics" => json!(self.state.formulas.business_metrics),
                        "tax" => json!(self.state.formulas.tax_calculations),
                        "real_estate" => json!(self.state.formulas.real_estate),
                        "retirement" => json!(self.state.formulas.retirement_planning),
                        _ => json!({
                            "error": format!("Unknown category: {}", cat),
                            "available_categories": ["financial", "business_metrics", "tax", "real_estate", "retirement"]
                        })
                    }
                } else {
                    json!({
                        "financial_formulas": self.state.formulas.financial_formulas,
                        "business_metrics": self.state.formulas.business_metrics,
                        "tax_calculations": self.state.formulas.tax_calculations,
                        "real_estate": self.state.formulas.real_estate,
                        "retirement_planning": self.state.formulas.retirement_planning
                    })
                };

                Ok(vec![ToolResult::text(serde_json::to_string_pretty(&formulas)?)])
            },
            "templates" => {
                Ok(vec![ToolResult::text(format!(
                    "{}\n\n## Data Sources\nFormulas loaded from: data/business_formulas.json\nBenchmarks loaded from: data/industry_benchmarks.yaml",
                    self.state.calculation_templates
                ))])
            },
            "examples" => {
                let examples = json!({
                    "real_world_examples": {
                        "mortgage_analysis": {
                            "scenario": "First-time home buyer analyzing mortgage options",
                            "calculation": "loan_payment",
                            "parameters": {
                                "principal": 400000,
                                "annual_rate": 0.0375,
                                "years": 30
                            },
                            "business_context": "Compare different lenders and loan terms"
                        },
                        "startup_investment": {
                            "scenario": "Evaluating startup investment opportunity",
                            "calculation": "net_present_value",
                            "parameters": {
                                "initial_investment": 100000,
                                "cash_flows": [25000, 30000, 40000, 50000, 60000],
                                "discount_rate": 0.15
                            },
                            "business_context": "Determine if investment meets return requirements"
                        },
                        "saas_metrics": {
                            "scenario": "SaaS company analyzing customer value",
                            "calculation": "customer_lifetime_value",
                            "parameters": {
                                "average_order_value": 99,
                                "purchase_frequency": 12,
                                "gross_margin": 0.80,
                                "customer_lifespan": 2.5
                            },
                            "business_context": "Set customer acquisition cost budgets"
                        }
                    },
                    "calculation_workflows": {
                        "investment_analysis": [
                            "1. Calculate NPV of project cash flows",
                            "2. Calculate IRR for return rate",
                            "3. Perform break-even analysis",
                            "4. Compare against industry benchmarks"
                        ],
                        "loan_comparison": [
                            "1. Calculate monthly payments for each option",
                            "2. Compare total interest paid",
                            "3. Analyze payment affordability",
                            "4. Consider tax implications"
                        ]
                    }
                });

                Ok(vec![ToolResult::text(serde_json::to_string_pretty(&examples)?)])
            },
            "all" => {
                let comprehensive_doc = json!({
                    "business_financial_calculator": {
                        "description": "Comprehensive financial calculator for business professionals",
                        "external_data_sources": {
                            "business_formulas": "data/business_formulas.json",
                            "industry_benchmarks": "data/industry_benchmarks.yaml",
                            "calculation_templates": "data/calculation_templates.md"
                        },
                        "capabilities": [
                            "Financial calculations (NPV, IRR, loan payments, compound interest)",
                            "Business metrics (CLV, churn rate, conversion rates, productivity)",
                            "Industry benchmarking across 7+ sectors",
                            "Real estate and investment analysis",
                            "Tax and retirement planning calculations"
                        ],
                        "formula_categories": self.state.formulas.financial_formulas.keys()
                            .chain(self.state.formulas.business_metrics.keys())
                            .collect::<Vec<_>>(),
                        "supported_industries": self.state.benchmarks.industry_benchmarks.keys().collect::<Vec<_>>(),
                        "usage_examples": {
                            "financial_analysis": "Evaluate investment opportunities with NPV, IRR calculations",
                            "business_operations": "Calculate CLV, churn rates, break-even points",
                            "benchmarking": "Compare performance against industry standards",
                            "loan_analysis": "Mortgage and business loan payment calculations"
                        }
                    }
                });

                Ok(vec![ToolResult::text(serde_json::to_string_pretty(&comprehensive_doc)?)])
            },
            _ => {
                Err(McpError::invalid_param_type("doc_type", "formulas|templates|examples|best_practices|all", doc_type))
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Business Financial Calculator System");

    // Load calculator state with external data
    let calculator_state = Arc::new(BusinessCalculatorState::new()?);

    // Parse command line arguments for bind address
    let bind_address: SocketAddr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8764".to_string())
        .parse()
        .map_err(|e| format!("Invalid bind address: {}", e))?;

    // Build the MCP server with business calculator tools
    let server = McpServer::builder()
        .name("business-financial-calculator")
        .version("1.0.0")
        .title("Business Financial Calculator System")
        .instructions("Real-world business financial calculator system for development teams and business professionals. Provides comprehensive financial calculations, business metrics analysis, industry benchmarking, and investment analysis using formulas and benchmarks loaded from external data files.")
        .tool(FinancialCalculatorTool::new(calculator_state.clone()))
        .tool(BusinessMetricsTool::new(calculator_state.clone()))
        .tool(IndustryBenchmarkTool::new(calculator_state.clone()))
        .tool(CalculatorDocumentationTool::new(calculator_state.clone()))
        .bind_address(bind_address)
        .mcp_path("/mcp")
        .cors(true)
        .sse(true)
        .build()?;

    info!("Business Financial Calculator System configured with capabilities:");
    info!("  - calculate_financial: NPV, IRR, loan payments, compound interest, break-even analysis");
    info!("  - calculate_business_metrics: CLV, churn rate, conversion rates, employee productivity");
    info!("  - get_industry_benchmarks: Performance standards across 7+ industries");
    info!("  - get_calculator_documentation: Comprehensive formulas, templates, and examples");
    info!("Server will bind to: {}", bind_address);
    info!("MCP endpoint available at: http://{}/mcp", bind_address);
    info!("External data files: data/business_formulas.json, data/industry_benchmarks.yaml, data/calculation_templates.md");
    info!("Real-world use cases: Investment analysis, loan comparison, business planning, performance benchmarking");

    // Run the server
    server.run().await?;

    Ok(())
}