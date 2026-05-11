#![forbid(unsafe_code)]
#![doc = "Planner model skeleton for runfossil."]

use std::fmt;

use runfossil_core::{CoverageDecision, PlanDecision, Priority, SourceSlug};

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
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_plan_has_no_tasks() {
        let plan = CapturePlan::empty();
        assert!(plan.tasks().is_empty());
    }
}
