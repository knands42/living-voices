use slint::{ModelRc, SharedString, VecModel};

use crate::backend::domain::adventure::Adventure;
use crate::backend::domain::memory::{Act, Fact, State};
use crate::ui::{ActView, AdventureView, FactView, StateView};

pub fn adventure_view(adventure: Option<&Adventure>) -> AdventureView {
    match adventure {
        Some(adventure) => AdventureView {
            id: SharedString::from(adventure.id.to_string()),
            title: SharedString::from(adventure.title.clone()),
            headline: SharedString::from(adventure.headline.clone()),
            tone: SharedString::from(adventure.tone.clone()),
        },
        None => AdventureView::default(),
    }
}

pub fn act_view(act: Option<&Act>) -> ActView {
    match act {
        Some(act) => ActView {
            goal: SharedString::from(act.goal.clone()),
            status: SharedString::from(act.status.to_string()),
            ordinal: act.ordinal as i32,
        },
        None => ActView::default(),
    }
}

pub fn state_view(state: Option<&State>) -> StateView {
    match state {
        Some(state) => StateView {
            day: SharedString::from(state.day.clone()),
            place: SharedString::from(state.place.clone()),
            characters_present: string_model(&state.characters_present),
            open_threads: string_model(&state.open_threads),
            conditions: string_model(&state.conditions),
        },
        None => StateView::default(),
    }
}

pub fn facts_model(facts: &[Fact]) -> ModelRc<FactView> {
    let items: Vec<FactView> = facts.iter().map(fact_view).collect();
    ModelRc::new(VecModel::from(items))
}

fn fact_view(fact: &Fact) -> FactView {
    FactView {
        subject: SharedString::from(fact.subject.clone()),
        fact: SharedString::from(fact.fact.clone()),
        permanence: SharedString::from(fact.permanence.to_string()),
    }
}

fn string_model(values: &[String]) -> ModelRc<SharedString> {
    let items: Vec<SharedString> = values.iter().map(SharedString::from).collect();
    ModelRc::new(VecModel::from(items))
}
