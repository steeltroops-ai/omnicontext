//! Plan auditing engine.
//!
//! Analyzes a task plan for architectural risks by computing blast radius,
//! co-change warnings, breaking change detection, and cycle analysis.

use std::collections::HashSet;
use std::fmt::Write;

use serde::{Deserialize, Serialize};

use crate::commits::CommitEngine;
use crate::error::OmniResult;
use crate::pipeline::Engine;
use crate::search::QueryIntent;

/// Risk level for plan analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    /// Few or no downstream dependents affected.
    Low,
    /// Some downstream dependents, manageable impact.
    Medium,
    /// Many downstream dependents, significant impact.
    High,
    /// Very large blast radius, critical caution required.
    Critical,
}

impl RiskLevel {
    fn from_blast_radius(count: usize) -> Self {
        if count > 50 {
            Self::Critical
        } else if count > 20 {
            Self::High
        } else if count > 5 {
            Self::Medium
        } else {
            Self::Low
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Critical => "Critical",
        }
    }

    fn emoji(self) -> &'static str {
        match self {
            Self::Low => "&#x2705;",
            Self::Medium => "&#x26A0;&#xFE0F;",
            Self::High => "&#x1F6A8;",
            Self::Critical => "&#x1F4A5;",
        }
    }
}

/// Analysis of a single step in the plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepAnalysis {
    /// The original step text.
    pub step_text: String,
    /// Classified intent of the step.
    pub intent: String,
    /// Files likely affected by this step.
    pub affected_files: Vec<String>,
    /// Symbols likely affected by this step.
    pub affected_symbols: Vec<String>,
    /// Total downstream dependents (blast radius).
    pub blast_radius: usize,
    /// Risk level for this step.
    pub risk: RiskLevel,
}

/// A potential breaking change identified in the plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakingChange {
    /// The symbol that may break.
    pub symbol: String,
    /// Dependents that would be affected.
    pub dependents: Vec<String>,
    /// Reason this is a breaking change.
    pub reason: String,
}

/// Warning about co-change patterns not addressed in the plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoChangeWarning {
    /// File mentioned in the plan.
    pub file: String,
    /// Files that frequently change with it (path, count).
    pub co_changes_with: Vec<(String, usize)>,
    /// Recommendation text.
    pub recommendation: String,
}

/// Complete plan critique result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanCritique {
    /// Per-step analysis.
    pub steps: Vec<StepAnalysis>,
    /// Overall risk level (max of all steps).
    pub overall_risk: RiskLevel,
    /// Potential breaking changes.
    pub breaking_changes: Vec<BreakingChange>,
    /// Co-change warnings (files missing from the plan).
    pub co_change_warnings: Vec<CoChangeWarning>,
    /// Cycle warnings in the dependency graph.
    pub cycle_warnings: Vec<String>,
    /// Human-readable summary.
    pub summary: String,
}

impl PlanCritique {
    /// Render the critique as markdown.
    pub fn to_markdown(&self) -> String {
        let mut out = String::with_capacity(2048);

        writeln!(
            out,
            "# Plan Audit Report\n\n**Overall Risk**: {} {}\n",
            self.overall_risk.emoji(),
            self.overall_risk.as_str()
        )
        .ok();

        writeln!(out, "## Summary\n\n{}\n", self.summary).ok();

        // Steps
        if !self.steps.is_empty() {
            writeln!(out, "## Step Analysis\n").ok();
            for (i, step) in self.steps.iter().enumerate() {
                writeln!(
                    out,
                    "### Step {} — {} {}\n",
                    i + 1,
                    step.risk.emoji(),
                    step.risk.as_str()
                )
                .ok();
                writeln!(
                    out,
                    "> {}\n",
                    step.step_text.chars().take(200).collect::<String>()
                )
                .ok();
                writeln!(out, "- **Intent**: {}", step.intent).ok();
                writeln!(out, "- **Blast radius**: {} dependents", step.blast_radius).ok();
                if !step.affected_files.is_empty() {
                    writeln!(
                        out,
                        "- **Files**: {}",
                        step.affected_files
                            .iter()
                            .take(5)
                            .map(|f| format!("`{f}`"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                    .ok();
                }
                if !step.affected_symbols.is_empty() {
                    writeln!(
                        out,
                        "- **Symbols**: {}",
                        step.affected_symbols
                            .iter()
                            .take(5)
                            .map(|s| format!("`{s}`"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                    .ok();
                }
                out.push('\n');
            }
        }

        // Breaking changes
        if !self.breaking_changes.is_empty() {
            writeln!(out, "## Breaking Changes\n").ok();
            for bc in &self.breaking_changes {
                writeln!(
                    out,
                    "- **{}**: {} ({} dependents)",
                    bc.symbol,
                    bc.reason,
                    bc.dependents.len()
                )
                .ok();
            }
            out.push('\n');
        }

        // Co-change warnings
        if !self.co_change_warnings.is_empty() {
            writeln!(out, "## Co-Change Warnings\n").ok();
            writeln!(
                out,
                "These files frequently change together but are **not mentioned** in the plan:\n"
            )
            .ok();
            for w in &self.co_change_warnings {
                writeln!(out, "- `{}` often changes with:", w.file).ok();
                for (co, count) in &w.co_changes_with {
                    writeln!(out, "  - `{co}` ({count} shared commits)").ok();
                }
                writeln!(out, "  *{}*\n", w.recommendation).ok();
            }
        }

        // Cycle warnings
        if !self.cycle_warnings.is_empty() {
            writeln!(out, "## Cycle Warnings\n").ok();
            for w in &self.cycle_warnings {
                writeln!(out, "- {w}").ok();
            }
            out.push('\n');
        }

        out
    }
}

/// Plan auditor that analyzes plans against the codebase.
pub struct PlanAuditor<'a> {
    engine: &'a Engine,
}

impl<'a> PlanAuditor<'a> {
    /// Create a new plan auditor.
    pub fn new(engine: &'a Engine) -> Self {
        Self { engine }
    }

    /// Audit a plan and return a structured critique.
    pub fn audit(&self, plan_text: &str, max_depth: usize) -> OmniResult<PlanCritique> {
        // 1. Parse plan into steps
        let steps = Self::parse_steps(plan_text);

        // 2. Analyze each step
        let mut step_analyses = Vec::new();
        let mut all_affected_files: HashSet<String> = HashSet::new();
        let mut all_affected_symbols: HashSet<String> = HashSet::new();

        for step in &steps {
            let analysis = self.analyze_step(step, max_depth)?;
            for f in &analysis.affected_files {
                all_affected_files.insert(f.clone());
            }
            for s in &analysis.affected_symbols {
                all_affected_symbols.insert(s.clone());
            }
            step_analyses.push(analysis);
        }

        // 3. Breaking changes: symbols with large blast radius
        let breaking_changes = self.find_breaking_changes(&all_affected_symbols, max_depth)?;

        // 4. Co-change warnings
        let co_change_warnings = self.find_co_change_warnings(&all_affected_files)?;

        // 5. Cycle warnings
        let cycle_warnings = self.check_cycles();

        // 6. Overall risk
        let overall_risk = step_analyses
            .iter()
            .map(|s| s.risk)
            .max()
            .unwrap_or(RiskLevel::Low);

        // 7. Summary
        let total_blast = step_analyses.iter().map(|s| s.blast_radius).sum::<usize>();
        let summary = format!(
            "Plan has {} steps affecting {} files and {} symbols. \
             Total blast radius: {} downstream dependents. \
             {} breaking change(s) detected, {} co-change warning(s).",
            step_analyses.len(),
            all_affected_files.len(),
            all_affected_symbols.len(),
            total_blast,
            breaking_changes.len(),
            co_change_warnings.len(),
        );

        Ok(PlanCritique {
            steps: step_analyses,
            overall_risk,
            breaking_changes,
            co_change_warnings,
            cycle_warnings,
            summary,
        })
    }

    /// Parse plan text into individual steps.
    fn parse_steps(plan_text: &str) -> Vec<String> {
        let mut steps = Vec::new();
        let mut current_step = String::new();

        for line in plan_text.lines() {
            let trimmed = line.trim();

            // Detect step boundaries
            let is_new_step = trimmed.starts_with("- ")
                || trimmed.starts_with("* ")
                || trimmed.starts_with("## ")
                || (trimmed.len() > 2
                    && trimmed.chars().next().is_some_and(|c| c.is_ascii_digit())
                    && trimmed.contains(". "));

            if is_new_step {
                if !current_step.trim().is_empty() {
                    steps.push(current_step.trim().to_string());
                }
                // Strip leading markers
                let clean = trimmed
                    .trim_start_matches("- ")
                    .trim_start_matches("* ")
                    .trim_start_matches("## ");
                // Strip numbered list prefix
                let clean = if let Some(idx) = clean.find(". ") {
                    let prefix = &clean[..idx];
                    if prefix.chars().all(|c| c.is_ascii_digit()) {
                        &clean[idx + 2..]
                    } else {
                        clean
                    }
                } else {
                    clean
                };
                current_step = clean.to_string();
            } else if !trimmed.is_empty() {
                if !current_step.is_empty() {
                    current_step.push(' ');
                }
                current_step.push_str(trimmed);
            }
        }

        if !current_step.trim().is_empty() {
            steps.push(current_step.trim().to_string());
        }

        // If no steps found, treat the whole plan as one step
        if steps.is_empty() && !plan_text.trim().is_empty() {
            steps.push(plan_text.trim().to_string());
        }

        steps
    }

    /// Analyze a single plan step.
    fn analyze_step(&self, step_text: &str, max_depth: usize) -> OmniResult<StepAnalysis> {
        let intent = QueryIntent::classify(step_text);
        let intent_str = format!("{intent:?}");

        // Search for relevant code
        let results = self.engine.search(step_text, 10).unwrap_or_default();

        let mut affected_files: Vec<String> = Vec::new();
        let mut affected_symbols: Vec<String> = Vec::new();
        let mut total_blast = 0_usize;

        // Track seen files/symbols for dedup
        let mut seen_files: HashSet<String> = HashSet::new();
        let mut seen_symbols: HashSet<String> = HashSet::new();

        for result in &results {
            let file_str = result.file_path.display().to_string();
            if seen_files.insert(file_str.clone()) {
                affected_files.push(file_str);
            }

            let sym = &result.chunk.symbol_path;
            if !sym.is_empty() && seen_symbols.insert(sym.clone()) {
                affected_symbols.push(sym.clone());
            }
        }

        // Compute blast radius for each mentioned symbol
        let index = self.engine.metadata_index();
        let graph = self.engine.dep_graph();

        for sym_name in &affected_symbols {
            if let Ok(Some(symbol)) = index.get_symbol_by_fqn(sym_name) {
                if let Ok(radius) = graph.blast_radius(symbol.id, max_depth) {
                    total_blast += radius.len();
                }
            }
        }

        let risk = RiskLevel::from_blast_radius(total_blast);

        Ok(StepAnalysis {
            step_text: step_text.to_string(),
            intent: intent_str,
            affected_files,
            affected_symbols,
            blast_radius: total_blast,
            risk,
        })
    }

    /// Find potential breaking changes in affected symbols.
    fn find_breaking_changes(
        &self,
        affected_symbols: &HashSet<String>,
        max_depth: usize,
    ) -> OmniResult<Vec<BreakingChange>> {
        let mut breaking = Vec::new();
        let index = self.engine.metadata_index();
        let graph = self.engine.dep_graph();

        for sym_name in affected_symbols {
            let symbol = match index.get_symbol_by_fqn(sym_name) {
                Ok(Some(s)) => s,
                _ => continue,
            };

            let radius = graph.blast_radius(symbol.id, max_depth).unwrap_or_default();
            if radius.len() > 10 {
                let dependents: Vec<String> = radius
                    .iter()
                    .take(10)
                    .filter_map(|(id, _)| index.get_symbol_by_id(*id).ok().flatten().map(|s| s.fqn))
                    .collect();

                breaking.push(BreakingChange {
                    symbol: sym_name.clone(),
                    dependents,
                    reason: format!(
                        "Modifying this symbol affects {} downstream dependents",
                        radius.len()
                    ),
                });
            }
        }

        Ok(breaking)
    }

    /// Find co-change partners not mentioned in the plan.
    fn find_co_change_warnings(
        &self,
        affected_files: &HashSet<String>,
    ) -> OmniResult<Vec<CoChangeWarning>> {
        let mut warnings = Vec::new();
        let index = self.engine.metadata_index();

        for file in affected_files {
            let co_changes = CommitEngine::co_change_files(index, file, 2, 5).unwrap_or_default();

            let missing: Vec<(String, usize)> = co_changes
                .into_iter()
                .filter(|c| !affected_files.contains(&c.path))
                .map(|c| (c.path, c.shared_commits))
                .collect();

            if !missing.is_empty() {
                warnings.push(CoChangeWarning {
                    file: file.clone(),
                    co_changes_with: missing.clone(),
                    recommendation: format!(
                        "Consider whether {} also need changes",
                        missing
                            .iter()
                            .map(|(p, _)| format!("`{p}`"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                });
            }
        }

        Ok(warnings)
    }

    /// Check for dependency cycles.
    fn check_cycles(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        let graph = self.engine.dep_graph();

        if graph.has_cycles() {
            if let Ok(cycles) = graph.find_cycles() {
                let index = self.engine.metadata_index();
                for cycle in cycles.iter().take(3) {
                    let names: Vec<String> = cycle
                        .iter()
                        .take(5)
                        .filter_map(|id| index.get_symbol_by_id(*id).ok().flatten().map(|s| s.fqn))
                        .collect();
                    if !names.is_empty() {
                        warnings.push(format!(
                            "Circular dependency detected: {} (cycle of {} symbols)",
                            names.join(" -> "),
                            cycle.len()
                        ));
                    }
                }
            }
        }

        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_steps_numbered() {
        let plan = "1. Modify the Embedder struct\n2. Update config.rs\n3. Add cache invalidation";
        let steps = PlanAuditor::parse_steps(plan);
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0], "Modify the Embedder struct");
        assert_eq!(steps[1], "Update config.rs");
    }

    #[test]
    fn test_parse_steps_bullet() {
        let plan = "- Modify the Embedder\n- Update config\n- Add tests";
        let steps = PlanAuditor::parse_steps(plan);
        assert_eq!(steps.len(), 3);
    }

    #[test]
    fn test_parse_steps_headers() {
        let plan = "## Step 1\nDo something\n## Step 2\nDo another thing";
        let steps = PlanAuditor::parse_steps(plan);
        assert_eq!(steps.len(), 2);
    }

    #[test]
    fn test_risk_level_from_blast_radius() {
        assert_eq!(RiskLevel::from_blast_radius(0), RiskLevel::Low);
        assert_eq!(RiskLevel::from_blast_radius(5), RiskLevel::Low);
        assert_eq!(RiskLevel::from_blast_radius(10), RiskLevel::Medium);
        assert_eq!(RiskLevel::from_blast_radius(30), RiskLevel::High);
        assert_eq!(RiskLevel::from_blast_radius(60), RiskLevel::Critical);
    }

    #[test]
    fn test_critique_to_markdown() {
        let critique = PlanCritique {
            steps: vec![StepAnalysis {
                step_text: "Update the parser".to_string(),
                intent: "Edit".to_string(),
                affected_files: vec!["src/parser.rs".to_string()],
                affected_symbols: vec!["parse_file".to_string()],
                blast_radius: 15,
                risk: RiskLevel::Medium,
            }],
            overall_risk: RiskLevel::Medium,
            breaking_changes: vec![],
            co_change_warnings: vec![],
            cycle_warnings: vec![],
            summary: "Plan has 1 step affecting 1 file.".to_string(),
        };

        let md = critique.to_markdown();
        assert!(md.contains("Plan Audit Report"));
        assert!(md.contains("Medium"));
        assert!(md.contains("parser"));
    }
}
