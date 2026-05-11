use runfossil_core::{CoverageDecision, PlanDecision, Priority, SourceSlug};

use crate::probe::HostProbe;
use crate::registry::CoverageUnit;
use crate::{
    CapturePlan, CapturePlanProbes, PlannedTask, Pressure, PressureLevels, RiskLevel, TaskLimits,
};

const PLANNER_ID: &str = "runfossil-adaptive-v0";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Scale {
    Small,
    Medium,
    Large,
}

impl Scale {
    fn as_str(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
        }
    }
}

/// Builds a capture plan from host probe results.
///
/// The planner applies coverage decisions, pressure policy, scale policy,
/// and root requirement checks to produce a stable set of planned tasks.
pub fn build_plan(probe: HostProbe) -> CapturePlan {
    let pressure = probe.pressure;
    let scale = estimate_scale(&probe);
    let probes = CapturePlanProbes {
        root: probe.root,
        cgroup_version: probe.cgroup_version.clone(),
        systemd_detected: probe.systemd_detected,
        container_runtime_sockets: probe.container_runtime_sockets.clone(),
    };
    let mut plan = CapturePlan::new(PLANNER_ID, probes, pressure);

    let registry = crate::registry::coverage_registry();

    for unit in &registry {
        let decision = determine_decision(unit, &probe, pressure, scale);
        let reason = build_reason(unit, decision, &probe, pressure, scale);
        let limits = compute_limits(unit.priority, decision, pressure, scale);
        let risk = unit_risk(unit);

        let task = PlannedTask {
            id: unit.id.clone(),
            source: unit.source,
            domain: unit.domain.clone(),
            object: unit.object.clone(),
            coverage_decision: unit.coverage_decision,
            decision,
            priority: unit.priority,
            risk,
            reason,
            limits,
            dependencies: Vec::new(),
        };

        plan.push(task);
    }

    plan
}

fn determine_decision(
    unit: &CoverageUnit,
    probe: &HostProbe,
    pressure: PressureLevels,
    scale: Scale,
) -> PlanDecision {
    if unit.coverage_decision == CoverageDecision::Exclude {
        return PlanDecision::SkippedByPolicy;
    }

    if unit.requires_root && !probe.root {
        return PlanDecision::SkippedByPolicy;
    }

    if !source_present(unit, probe) {
        return PlanDecision::NotPresent;
    }

    let pressure_decision = apply_pressure_policy(unit, pressure);
    if pressure_decision != PlanDecision::Scheduled {
        return pressure_decision;
    }

    let scale_decision = apply_scale_policy(unit, scale);
    if scale_decision != PlanDecision::Scheduled {
        return scale_decision;
    }

    if unit.coverage_decision == CoverageDecision::DeferredNative {
        return PlanDecision::Unsupported;
    }

    match unit.coverage_decision {
        CoverageDecision::Limited => PlanDecision::Limited,
        CoverageDecision::Conditional => {
            if conditions_met(unit, probe) {
                PlanDecision::Scheduled
            } else {
                PlanDecision::SkippedByPolicy
            }
        }
        _ => PlanDecision::Scheduled,
    }
}

fn source_present(unit: &CoverageUnit, probe: &HostProbe) -> bool {
    match unit.source {
        SourceSlug::Proc => true,
        SourceSlug::Sys => match unit.domain.as_str() {
            "cgroup" => probe.cgroup_version.is_some(),
            "pstore" => probe.pstore_available,
            "debugfs" => probe.debugfs_available,
            "tracefs" => probe.tracefs_available,
            "bpffs" => probe.bpffs_available,
            "securityfs" => probe.securityfs_available,
            "firmware" => probe.root,
            _ => true,
        },
        SourceSlug::Run => match unit.domain.as_str() {
            "systemd_runtime" => probe.systemd_detected,
            _ => true,
        },
        SourceSlug::Dev => true,
        SourceSlug::Kernel => true,
        SourceSlug::Netlink => probe.root,
        SourceSlug::Service => probe.systemd_detected,
        SourceSlug::Container => {
            !probe.container_runtime_sockets.is_empty() || container_evidence_present()
        }
        SourceSlug::Logs => probe.systemd_detected,
        SourceSlug::Sessions => probe.systemd_detected,
        SourceSlug::Time => true,
        SourceSlug::Security => probe.root,
        SourceSlug::Scheduler => true,
        SourceSlug::Crash => probe.pstore_available,
        SourceSlug::Hardware => true,
    }
}

fn apply_pressure_policy(unit: &CoverageUnit, pressure: PressureLevels) -> PlanDecision {
    let worst = worst_pressure(pressure);

    match (unit.priority, worst) {
        (Priority::P4, Pressure::Critical | Pressure::High) => PlanDecision::SkippedByPolicy,
        (Priority::P4, Pressure::Moderate) => PlanDecision::Limited,
        (Priority::P3, Pressure::Critical) => PlanDecision::SkippedByPolicy,
        (Priority::P3, Pressure::High) => PlanDecision::Limited,
        _ => PlanDecision::Scheduled,
    }
}

fn apply_scale_policy(unit: &CoverageUnit, scale: Scale) -> PlanDecision {
    match (unit.priority, scale) {
        (Priority::P4, Scale::Large) => PlanDecision::Limited,
        _ => PlanDecision::Scheduled,
    }
}

fn conditions_met(unit: &CoverageUnit, probe: &HostProbe) -> bool {
    match unit.id.as_str() {
        "sys.firmware" | "sys.securityfs" | "sys.class_drm" => probe.root,
        "run.systemd" => probe.systemd_detected,
        "security.selinux" => probe.root,
        "container.process"
        | "container.resources"
        | "container.namespace"
        | "container.cgroup" => probe.root && container_evidence_present(),
        _ => true,
    }
}

fn worst_pressure(pressure: PressureLevels) -> Pressure {
    let mut worst = pressure.cpu;
    if pressure.memory > worst {
        worst = pressure.memory;
    }
    if pressure.io > worst {
        worst = pressure.io;
    }
    worst
}

fn container_evidence_present() -> bool {
    use std::path::Path;

    Path::new("/run/runc").is_dir()
        || Path::new("/run/docker").is_dir()
        || Path::new("/run/containerd").is_dir()
        || Path::new("/run/crio").is_dir()
        || Path::new("/sys/fs/cgroup/system.slice").is_dir()
}

fn build_reason(
    unit: &CoverageUnit,
    decision: PlanDecision,
    probe: &HostProbe,
    _pressure: PressureLevels,
    scale: Scale,
) -> String {
    match decision {
        PlanDecision::Scheduled if unit.coverage_decision == CoverageDecision::Conditional => {
            format!(
                "conditional source scheduled (scale={}, root={})",
                scale.as_str(),
                probe.root
            )
        }
        PlanDecision::Scheduled => format!("source present (scale={})", scale.as_str()),
        PlanDecision::Limited => format!(
            "limited by policy (priority={}, scale={})",
            unit.priority.as_str(),
            scale.as_str()
        ),
        PlanDecision::SkippedByPolicy => {
            if unit.requires_root && !probe.root {
                format!(
                    "skipped: requires root (current euid is not 0, priority={})",
                    unit.priority.as_str()
                )
            } else if unit.coverage_decision == CoverageDecision::Exclude {
                "excluded from core scope".to_string()
            } else if unit.coverage_decision == CoverageDecision::Conditional
                && !conditions_met(unit, probe)
            {
                format!(
                    "conditional source not selected (conditions not met, priority={})",
                    unit.priority.as_str()
                )
            } else {
                format!(
                    "skipped by policy (priority={}, scale={})",
                    unit.priority.as_str(),
                    scale.as_str()
                )
            }
        }
        PlanDecision::Unsupported => {
            format!(
                "unsupported: native collector not yet implemented (priority={})",
                unit.priority.as_str()
            )
        }
        PlanDecision::NotPresent => {
            format!(
                "source or object not present on this host (domain={})",
                unit.domain
            )
        }
    }
}

fn compute_limits(
    priority: Priority,
    decision: PlanDecision,
    pressure: PressureLevels,
    scale: Scale,
) -> TaskLimits {
    let worst = worst_pressure(pressure);

    let (base_bytes, base_timeout_ms, base_files, base_depth) = match priority {
        Priority::P0 => (65_536, 5_000, 0, 0),
        Priority::P1 => (131_072, 5_000, 64, 2),
        Priority::P2 => (262_144, 10_000, 32, 3),
        Priority::P3 => (1_048_576, 15_000, 16, 2),
        Priority::P4 => (65_536, 5_000, 4, 1),
        Priority::NotApplicable => (0, 0, 0, 0),
    };

    if decision == PlanDecision::Limited || decision == PlanDecision::SkippedByPolicy {
        return TaskLimits::new(0, 0, 0, 0);
    }

    let pressure_factor: f64 = match worst {
        Pressure::Low => 1.0,
        Pressure::Moderate => 0.7,
        Pressure::High => 0.4,
        Pressure::Critical => 0.2,
    };

    let scale_factor: f64 = match scale {
        Scale::Small => 1.0,
        Scale::Medium => 0.8,
        Scale::Large => 0.5,
    };

    let factor = f64::min(pressure_factor, scale_factor);

    let max_bytes = (base_bytes as f64 * factor) as u64;
    let timeout_ms = (base_timeout_ms as f64 * factor) as u64;
    let max_files = (base_files as f64 * factor) as u64;
    let max_depth = (base_depth as f64 * factor) as u32;

    TaskLimits::new(max_bytes, timeout_ms, max_files, max_depth)
}

fn unit_risk(unit: &CoverageUnit) -> RiskLevel {
    match unit.coverage_decision {
        CoverageDecision::Exclude => RiskLevel::PolicySensitive,
        CoverageDecision::DeferredNative => RiskLevel::Unstable,
        CoverageDecision::Limited => RiskLevel::HighCost,
        CoverageDecision::Conditional => RiskLevel::Medium,
        CoverageDecision::Collect => match unit.priority {
            Priority::P4 => RiskLevel::HighCost,
            Priority::P3 => RiskLevel::Medium,
            _ => RiskLevel::Low,
        },
    }
}

fn estimate_scale(probe: &HostProbe) -> Scale {
    let cpu_weight = probe.cpu_count as u64;
    let mem_gb = probe.memory_total_kb / (1_048_576);
    let proc_weight = probe.process_count as u64;

    if cpu_weight > 64 || mem_gb > 256 || proc_weight > 1_000 {
        Scale::Large
    } else if cpu_weight > 8 || mem_gb > 32 || proc_weight > 200 {
        Scale::Medium
    } else {
        Scale::Small
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Pressure;
    use crate::probe::HostProbe;

    fn small_probe() -> HostProbe {
        let mut p = HostProbe::empty();
        p.root = true;
        p.cpu_count = 4;
        p.memory_total_kb = 8_388_608;
        p.process_count = 50;
        p.systemd_detected = true;
        p.cgroup_version = Some("v2".to_string());
        p.pstore_available = true;
        p.debugfs_available = true;
        p.tracefs_available = true;
        p.bpffs_available = true;
        p.securityfs_available = true;
        p
    }

    fn large_probe() -> HostProbe {
        let mut p = HostProbe::empty();
        p.root = true;
        p.cpu_count = 128;
        p.memory_total_kb = 524_288_000;
        p.process_count = 2_000;
        p.systemd_detected = true;
        p.cgroup_version = Some("v2".to_string());
        p.pstore_available = true;
        p
    }

    #[test]
    fn build_plan_from_small_host_has_tasks() {
        let plan = build_plan(small_probe());
        assert!(!plan.tasks().is_empty());
    }

    #[test]
    fn p0_collect_tasks_are_scheduled() {
        let plan = build_plan(small_probe());
        let p0: Vec<&PlannedTask> = plan
            .tasks()
            .iter()
            .filter(|t| t.id == "proc.loadavg" || t.id == "proc.meminfo")
            .collect();
        for task in p0 {
            assert_eq!(task.decision, PlanDecision::Scheduled);
        }
    }

    #[test]
    fn deferred_native_is_unsupported() {
        let plan = build_plan(small_probe());
        let deferred_tasks: Vec<&PlannedTask> = plan
            .tasks()
            .iter()
            .filter(|t| {
                matches!(t.coverage_decision, CoverageDecision::DeferredNative)
                    && t.decision != PlanDecision::NotPresent
            })
            .collect();
        assert!(!deferred_tasks.is_empty());
        for task in deferred_tasks {
            assert_eq!(task.decision, PlanDecision::Unsupported);
        }
    }

    #[test]
    fn root_required_task_skipped_when_not_root() {
        let mut probe = small_probe();
        probe.root = false;
        let plan = build_plan(probe);
        let iomem = plan
            .tasks()
            .iter()
            .find(|t| t.id == "proc.iomem")
            .expect("proc.iomem should be in plan");
        assert_eq!(iomem.decision, PlanDecision::SkippedByPolicy);
    }

    #[test]
    fn critical_pressure_skips_p3() {
        let mut probe = small_probe();
        probe.pressure =
            PressureLevels::new(Pressure::Critical, Pressure::Critical, Pressure::Critical);
        let plan = build_plan(probe);
        let p3_tasks: Vec<&PlannedTask> = plan
            .tasks()
            .iter()
            .filter(|t| t.priority == Priority::P3 && t.decision != PlanDecision::NotPresent)
            .collect();
        assert!(
            !p3_tasks.is_empty(),
            "should have at least one present P3 task"
        );
        for task in p3_tasks {
            assert_eq!(task.decision, PlanDecision::SkippedByPolicy);
        }
    }

    #[test]
    fn scale_large_is_large() {
        let probe = large_probe();
        assert_eq!(estimate_scale(&probe), Scale::Large);
    }

    #[test]
    fn container_source_not_present_without_sockets() {
        let mut probe = small_probe();
        probe.container_runtime_sockets = Vec::new();
        probe.root = false;
        let plan = build_plan(probe);
        let container_tasks: Vec<&PlannedTask> = plan
            .tasks()
            .iter()
            .filter(|t| t.source == SourceSlug::Container)
            .collect();
        for task in container_tasks {
            let expected = if task.coverage_decision == CoverageDecision::DeferredNative {
                PlanDecision::Unsupported
            } else {
                PlanDecision::SkippedByPolicy
            };
            assert_eq!(
                task.decision, expected,
                "container task {} expected {:?} but got {:?}",
                task.id, expected, task.decision
            );
        }
    }
}
