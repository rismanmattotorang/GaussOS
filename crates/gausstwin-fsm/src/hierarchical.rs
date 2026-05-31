use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::{ActionFn, GuardFn, Signal, State, StateId, StateMachine, TransitionId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchicalState {
    pub id: StateId,
    pub name: String,
    pub parent: Option<StateId>,
    pub children: HashSet<StateId>,
    pub entry_actions: Vec<ActionFn>,
    pub exit_actions: Vec<ActionFn>,
    pub data: Arc<dyn std::any::Any + Send + Sync>,
    pub orthogonal_regions: Vec<StateId>,
    pub deep_history: bool,
}

#[derive(Debug)]
pub struct HierarchicalTransition {
    pub id: TransitionId,
    pub from: StateId,
    pub to: StateId,
    pub guard: GuardFn,
    pub action: ActionFn,
    pub priority: i32,
    pub trigger_type: TriggerType,
    pub timeout: Option<std::time::Duration>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TriggerType {
    External,
    Internal,
    Completion,
    History,
}

pub struct HierarchicalFSM<C> {
    states: DashMap<StateId, HierarchicalState>,
    transitions: DashMap<TransitionId, HierarchicalTransition>,
    active_states: RwLock<HashSet<StateId>>,
    history: DashMap<StateId, StateId>,
    context: C,
    metrics: Arc<DashMap<String, f64>>,
    observers: tokio::sync::broadcast::Sender<Vec<StateId>>,
}

impl<C: Send + Sync> HierarchicalFSM<C> {
    pub fn new(initial_state: HierarchicalState, context: C) -> Self {
        let (tx, _) = tokio::sync::broadcast::channel(1024);
        let mut states = DashMap::new();
        states.insert(initial_state.id, initial_state);
        
        Self {
            states,
            transitions: DashMap::new(),
            active_states: RwLock::new(HashSet::new()),
            history: DashMap::new(),
            context,
            metrics: Arc::new(DashMap::new()),
            observers: tx,
        }
    }

    pub fn add_orthogonal_region(&mut self, parent_id: StateId, region_id: StateId) -> bool {
        if let Some(mut parent) = self.states.get_mut(&parent_id) {
            parent.orthogonal_regions.push(region_id);
            true
        } else {
            false
        }
    }

    pub fn get_active_states(&self) -> HashSet<StateId> {
        self.active_states.read().clone()
    }

    pub fn is_active(&self, state_id: StateId) -> bool {
        self.active_states.read().contains(&state_id)
    }

    fn enter_state(&self, state_id: StateId, ctx: &mut C) {
        if let Some(state) = self.states.get(&state_id) {
            // Enter parent states first if not already active
            if let Some(parent_id) = state.parent {
                if !self.is_active(parent_id) {
                    self.enter_state(parent_id, ctx);
                }
            }

            // Execute entry actions
            for action in &state.entry_actions {
                (action)(ctx as &mut dyn std::any::Any);
            }

            // Add to active states
            self.active_states.write().insert(state_id);

            // Enter orthogonal regions
            for region_id in &state.orthogonal_regions {
                self.enter_state(*region_id, ctx);
            }

            // Handle history
            if state.deep_history {
                if let Some(history_state) = self.history.get(&state_id) {
                    self.enter_state(*history_state, ctx);
                }
            }
        }
    }

    fn exit_state(&self, state_id: StateId, ctx: &mut C) {
        if let Some(state) = self.states.get(&state_id) {
            // Exit children first
            for child_id in &state.children {
                if self.is_active(*child_id) {
                    self.exit_state(*child_id, ctx);
                }
            }

            // Store history if needed
            if state.deep_history {
                if let Some(active_child) = state.children
                    .iter()
                    .find(|&child_id| self.is_active(*child_id))
                {
                    self.history.insert(state_id, *active_child);
                }
            }

            // Execute exit actions
            for action in &state.exit_actions {
                (action)(ctx as &mut dyn std::any::Any);
            }

            // Remove from active states
            self.active_states.write().remove(&state_id);
        }
    }

    pub fn get_enabled_transitions(&self, signal: &Signal) -> Vec<TransitionId> {
        let active_states = self.active_states.read();
        let mut enabled = Vec::new();

        for transition in self.transitions.iter() {
            if active_states.contains(&transition.from) && (transition.guard)(signal) {
                enabled.push(transition.id);
            }
        }

        // Sort by priority
        enabled.sort_by_key(|id| {
            -self.transitions.get(id).map_or(0, |t| t.priority)
        });

        enabled
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Vec<StateId>> {
        self.observers.subscribe()
    }

    pub fn get_metrics(&self) -> Arc<DashMap<String, f64>> {
        self.metrics.clone()
    }
}

impl<C: Send + Sync> StateMachine for HierarchicalFSM<C> {
    type Context = C;

    fn current(&self) -> StateId {
        // Return the leaf state in the main region
        let active_states = self.active_states.read();
        active_states.iter()
            .find(|&id| {
                if let Some(state) = self.states.get(id) {
                    state.children.is_empty() || 
                    !state.children.iter().any(|child| active_states.contains(child))
                } else {
                    false
                }
            })
            .copied()
            .unwrap_or_default()
    }

    fn transition(&mut self, signal: &Signal, ctx: &mut Self::Context) -> StateId {
        let enabled_transitions = self.get_enabled_transitions(signal);
        
        for transition_id in enabled_transitions {
            if let Some(transition) = self.transitions.get(&transition_id) {
                // Find LCA (Least Common Ancestor)
                let mut source_ancestors = Vec::new();
                let mut current = transition.from;
                while let Some(state) = self.states.get(&current) {
                    source_ancestors.push(current);
                    if let Some(parent) = state.parent {
                        current = parent;
                    } else {
                        break;
                    }
                }

                let mut target_ancestors = Vec::new();
                current = transition.to;
                while let Some(state) = self.states.get(&current) {
                    target_ancestors.push(current);
                    if let Some(parent) = state.parent {
                        current = parent;
                    } else {
                        break;
                    }
                }

                let lca = source_ancestors.iter()
                    .find(|&id| target_ancestors.contains(id))
                    .copied()
                    .unwrap_or_default();

                // Exit states up to LCA
                for state_id in source_ancestors.iter().take_while(|&id| *id != lca) {
                    self.exit_state(*state_id, ctx);
                }

                // Execute transition action
                (transition.action)(ctx as &mut dyn std::any::Any);

                // Enter states from LCA
                for state_id in target_ancestors.iter().rev().skip_while(|&id| *id != lca) {
                    self.enter_state(*state_id, ctx);
                }

                // Update metrics
                self.metrics.insert("transitions_total".into(),
                    self.metrics.get("transitions_total").map_or(1.0, |v| v + 1.0));

                // Notify observers
                let _ = self.observers.send(self.get_active_states().into_iter().collect());

                return transition.to;
            }
        }

        self.current()
    }

    fn add_state(&mut self, state: State) {
        let hierarchical_state = HierarchicalState {
            id: state.id,
            name: state.name,
            parent: None,
            children: HashSet::new(),
            entry_actions: state.entry_actions,
            exit_actions: state.exit_actions,
            data: state.data,
            orthogonal_regions: Vec::new(),
            deep_history: false,
        };
        self.states.insert(state.id, hierarchical_state);
    }

    fn add_transition(&mut self, transition: crate::Transition) {
        let hierarchical_transition = HierarchicalTransition {
            id: transition.id,
            from: transition.from,
            to: transition.to,
            guard: transition.guard,
            action: transition.action,
            priority: transition.priority,
            trigger_type: TriggerType::External,
            timeout: transition.timeout,
        };
        self.transitions.insert(transition.id, hierarchical_transition);
    }

    fn remove_state(&mut self, id: StateId) -> bool {
        self.states.remove(&id).is_some()
    }

    fn remove_transition(&mut self, id: TransitionId) -> bool {
        self.transitions.remove(&id).is_some()
    }
} 