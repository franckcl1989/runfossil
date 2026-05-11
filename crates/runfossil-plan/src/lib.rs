#![forbid(unsafe_code)]
#![doc = "Adaptive capture planner for runfossil."]

use std::fmt;

use runfossil_core::{CoverageDecision, PlanDecision, Priority, SourceSlug};

mod builder;
mod probe;
mod registry;

pub use builder::build_plan;
pub use probe::probe_host;

/// Planner risk level for a candidate capture task.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RiskLevel {
    /// Low-risk task.
    Low,
    /// Medium-risk task.
    Medium,
    /// High-cost task.
    HighCost,
    /// Policy-sensitive task.
    PolicySensitive,
    /// Unstable runtime source.
    Unstable,
}

impl RiskLevel {
    /// Returns the plan/control-file spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::HighCost => "high_cost",
            Self::PolicySensitive => "policy_sensitive",
            Self::Unstable => "unstable",
        }
    }
}

impl fmt::Display for RiskLevel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Effective limits assigned to a capture task.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TaskLimits {
    /// Maximum bytes to write for file-like payloads.
    pub max_bytes: u64,
    /// Timeout in milliseconds.
    pub timeout_ms: u64,
    /// Maximum files to visit for traversal tasks.
    pub max_files: u64,
    /// Maximum traversal depth.
    pub max_depth: u32,
}

impl TaskLimits {
    /// Creates task limits.
    #[must_use]
    pub const fn new(max_bytes: u64, timeout_ms: u64, max_files: u64, max_depth: u32) -> Self {
        Self {
            max_bytes,
            timeout_ms,
            max_files,
            max_depth,
        }
    }
}

/// A planned capture task.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedTask {
    /// Stable task identifier.
    pub id: String,
    /// L1 source family.
    pub source: SourceSlug,
    /// L2 source domain.
    pub domain: String,
    /// Runtime source identity or logical object name.
    pub object: String,
    /// Static coverage decision.
    pub coverage_decision: CoverageDecision,
    /// Runtime plan decision.
    pub decision: PlanDecision,
    /// Task priority.
    pub priority: Priority,
    /// Planner risk level.
    pub risk: RiskLevel,
    /// Explanation for the decision.
    pub reason: String,
    /// Effective task limits.
    pub limits: TaskLimits,
    /// Task IDs that influenced this task.
    pub dependencies: Vec<String>,
}

/// Runtime pressure levels detected during the planner probe phase.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PressureLevels {
    /// CPU pressure level.
    pub cpu: Pressure,
    /// Memory pressure level.
    pub memory: Pressure,
    /// I/O pressure level.
    pub io: Pressure,
}

impl PressureLevels {
    /// Creates pressure levels with the given values.
    #[must_use]
    pub const fn new(cpu: Pressure, memory: Pressure, io: Pressure) -> Self {
        Self { cpu, memory, io }
    }
}

/// Pressure level detected from PSI or other host signals.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub enum Pressure {
    /// Low pressure.
    #[default]
    Low,
    /// Moderate pressure.
    Moderate,
    /// High pressure.
    High,
    /// Critical pressure.
    Critical,
}

impl Pressure {
    /// Returns the plan/control-file spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Moderate => "moderate",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

impl fmt::Display for Pressure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Host probes used for planning decisions.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CapturePlanProbes {
    /// Whether effective UID is root.
    pub root: bool,
    /// Detected cgroup version.
    pub cgroup_version: Option<String>,
    /// Whether systemd service manager was detected.
    pub systemd_detected: bool,
    /// Container runtime sockets discovered on the host.
    pub container_runtime_sockets: Vec<String>,
}

impl CapturePlanProbes {
    /// Creates an empty probe result.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            root: false,
            cgroup_version: None,
            systemd_detected: false,
            container_runtime_sockets: Vec::new(),
        }
    }
}

/// Capture plan produced by the planner.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CapturePlan {
    planner: String,
    probes: CapturePlanProbes,
    pressure: PressureLevels,
    tasks: Vec<PlannedTask>,
}

impl CapturePlan {
    /// Creates a plan with the given strategy name and probes.
    #[must_use]
    pub fn new(
        planner: impl Into<String>,
        probes: CapturePlanProbes,
        pressure: PressureLevels,
    ) -> Self {
        Self {
            planner: planner.into(),
            probes,
            pressure,
            tasks: Vec::new(),
        }
    }

    /// Creates an empty plan.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            planner: String::new(),
            probes: CapturePlanProbes::empty(),
            pressure: PressureLevels::new(Pressure::Low, Pressure::Low, Pressure::Low),
            tasks: Vec::new(),
        }
    }

    /// Returns the planner strategy identifier.
    #[must_use]
    pub fn planner(&self) -> &str {
        &self.planner
    }

    /// Returns the host probes.
    #[must_use]
    pub fn probes(&self) -> &CapturePlanProbes {
        &self.probes
    }

    /// Returns the detected pressure levels.
    #[must_use]
    pub fn pressure(&self) -> &PressureLevels {
        &self.pressure
    }

    /// Adds a planned task.
    pub fn push(&mut self, task: PlannedTask) {
        self.tasks.push(task);
    }

    /// Returns planned tasks.
    #[must_use]
    pub fn tasks(&self) -> &[PlannedTask] {
        &self.tasks
    }

    /// Serializes the plan to a JSON string.
    pub fn to_json(&self) -> String {
        let mut buf = String::with_capacity(32_768);
        buf.push_str("{\n");
        buf.push_str(r#"  "planner": ""#);
        json_escape(&self.planner, &mut buf);
        buf.push_str("\",\n");

        buf.push_str(r#"  "probes": {"root": "#);
        buf.push_str(&format!("{}", self.probes.root));
        buf.push_str(&format!(
            r#", "cgroup_version": {}"#,
            json_optional_str(&self.probes.cgroup_version)
        ));
        buf.push_str(&format!(
            r#", "systemd_detected": {}"#,
            self.probes.systemd_detected
        ));
        buf.push_str(r#", "container_runtime_sockets": ["#);
        for (i, socket) in self.probes.container_runtime_sockets.iter().enumerate() {
            if i > 0 {
                buf.push_str(", ");
            }
            buf.push('"');
            json_escape(socket, &mut buf);
            buf.push('"');
        }
        buf.push_str("]},\n");

        buf.push_str(r#"  "pressure": {"#);
        buf.push_str(&format!(r#""cpu": "{}""#, self.pressure.cpu.as_str()));
        buf.push_str(&format!(
            r#", "memory": "{}""#,
            self.pressure.memory.as_str()
        ));
        buf.push_str(&format!(r#", "io": "{}""#, self.pressure.io.as_str()));
        buf.push_str("},\n");

        buf.push_str(r#"  "tasks": ["#);
        if self.tasks.is_empty() {
            buf.push(']');
        } else {
            buf.push('\n');
            for (i, task) in self.tasks.iter().enumerate() {
                buf.push_str("    {\n");
                buf.push_str(&format!(
                    r#"      "id": "{}""#,
                    json_escape_inline(&task.id)
                ));
                buf.push_str(&format!(r#", "source": "{}""#, task.source.as_str()));
                buf.push_str(&format!(
                    r#", "domain": "{}""#,
                    json_escape_inline(&task.domain)
                ));
                buf.push_str(&format!(
                    r#", "object": "{}""#,
                    json_escape_inline(&task.object)
                ));
                buf.push_str(&format!(
                    r#", "coverage_decision": "{}""#,
                    task.coverage_decision.as_str()
                ));
                buf.push_str(&format!(r#", "decision": "{}""#, task.decision.as_str()));
                buf.push_str(&format!(r#", "priority": "{}""#, task.priority.as_str()));
                buf.push_str(&format!(r#", "risk": "{}""#, task.risk.as_str()));
                buf.push_str(&format!(
                    r#", "reason": "{}""#,
                    json_escape_inline(&task.reason)
                ));
                buf.push_str(&format!(
                    r#", "limits": {{"max_bytes": {}, "timeout_ms": {}, "max_files": {}, "max_depth": {}}}"#,
                    task.limits.max_bytes,
                    task.limits.timeout_ms,
                    task.limits.max_files,
                    task.limits.max_depth,
                ));
                buf.push_str(r#", "dependencies": ["#);
                for (j, dep) in task.dependencies.iter().enumerate() {
                    if j > 0 {
                        buf.push_str(", ");
                    }
                    buf.push('"');
                    json_escape(dep, &mut buf);
                    buf.push('"');
                }
                buf.push(']');

                if i + 1 < self.tasks.len() {
                    buf.push_str("\n    },\n");
                } else {
                    buf.push_str("\n    }\n");
                }
            }
            buf.push_str("  ]");
        }

        buf.push_str("\n}\n");
        buf
    }
}

fn json_optional_str(value: &Option<String>) -> String {
    match value {
        Some(s) => format!("\"{}\"", native(s)),
        None => "null".to_string(),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn native<T: ToString>(value: T) -> String {
    value.to_string()
}

fn json_escape_inline(value: &str) -> String {
    let mut buf = String::new();
    json_escape(value, &mut buf);
    buf
}

fn json_escape(src: &str, buf: &mut String) {
    for ch in src.chars() {
        match ch {
            '"' => buf.push_str("\\\""),
            '\\' => buf.push_str("\\\\"),
            '\n' => buf.push_str("\\n"),
            '\r' => buf.push_str("\\r"),
            '\t' => buf.push_str("\\t"),
            c if c.is_control() => {
                buf.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => buf.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_plan_has_no_tasks() {
        let plan = CapturePlan::empty();
        assert!(plan.tasks().is_empty());
    }

    #[test]
    fn empty_plan_to_json_has_planner_field() {
        let plan = CapturePlan::empty();
        let json = plan.to_json();
        assert!(json.contains(r#""planner""#));
    }

    #[test]
    fn plan_with_tasks_to_json_has_tasks_array() {
        let mut plan = CapturePlan::empty();
        plan.push(PlannedTask {
            id: "test.task".to_string(),
            source: SourceSlug::Proc,
            domain: "test".to_string(),
            object: "test".to_string(),
            coverage_decision: CoverageDecision::Collect,
            decision: PlanDecision::Scheduled,
            priority: Priority::P0,
            risk: RiskLevel::Low,
            reason: "test reason".to_string(),
            limits: TaskLimits::new(4096, 1000, 0, 0),
            dependencies: vec![],
        });
        let json = plan.to_json();
        assert!(json.contains(r#""tasks""#));
        assert!(json.contains(r#""test.task""#));
    }

    #[test]
    fn to_json_escapes_control_characters() {
        let mut plan = CapturePlan::empty();
        plan.push(PlannedTask {
            id: "test.esc".to_string(),
            source: SourceSlug::Proc,
            domain: "dom".to_string(),
            object: "obj\"quote".to_string(),
            coverage_decision: CoverageDecision::Collect,
            decision: PlanDecision::Scheduled,
            priority: Priority::P0,
            risk: RiskLevel::Low,
            reason: "reason\nnewline".to_string(),
            limits: TaskLimits::new(0, 0, 0, 0),
            dependencies: vec![],
        });
        let json = plan.to_json();
        assert!(json.contains(r#"\"quote"#));
        assert!(json.contains("\\nnewline"));
    }
}
