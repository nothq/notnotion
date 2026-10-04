use std::{collections::HashMap, rc::Rc};

use gpui::{AnyWeakEntity, App, Context, EntityId, Window};

/// A redraw capability that exposes no entity state to resource loaders.
#[derive(Clone)]
pub(crate) struct ViewNotifier {
    owner: AnyWeakEntity,
}

impl ViewNotifier {
    pub(crate) fn new<Owner: 'static>(cx: &Context<Owner>) -> Self {
        Self {
            owner: cx.entity().downgrade().into(),
        }
    }

    pub(crate) fn is_alive(&self) -> bool {
        self.owner.is_upgradable()
    }

    pub(crate) fn notify(&self, cx: &mut App) {
        if self.is_alive() {
            cx.notify(self.owner.entity_id());
        }
    }
}

/// Deduplicates the live views waiting for one shared resource.
#[derive(Default)]
pub(crate) struct ViewNotifiers(HashMap<EntityId, ViewNotifier>);

impl ViewNotifiers {
    pub(crate) fn insert(&mut self, notifier: &ViewNotifier) {
        self.0.retain(|_, notifier| notifier.is_alive());
        self.0.insert(notifier.owner.entity_id(), notifier.clone());
    }

    pub(crate) fn notify(self, cx: &mut App) {
        for notifier in self.0.into_values() {
            notifier.notify(cx);
        }
    }
}

/// Lets a view send domain actions without access to its hosting entity's state.
pub(crate) struct ViewActionSink<Action> {
    send: Rc<ViewActionHandler<Action>>,
}

type ViewActionHandler<Action> = dyn Fn(Action, &mut Window, &mut App);

impl<Action> Clone for ViewActionSink<Action> {
    fn clone(&self) -> Self {
        Self {
            send: self.send.clone(),
        }
    }
}

impl<Action: 'static> ViewActionSink<Action> {
    pub(crate) fn new<Owner: 'static>(
        cx: &Context<Owner>,
        handle: impl Fn(&mut Owner, Action, &mut Window, &mut Context<Owner>) + 'static,
    ) -> Self {
        let owner = cx.entity().downgrade();
        Self {
            send: Rc::new(move |action, window, cx| {
                owner
                    .update(cx, |owner, cx| handle(owner, action, window, cx))
                    .ok();
            }),
        }
    }

    pub(crate) fn emit(&self, action: Action, window: &mut Window, cx: &mut App) {
        (self.send)(action, window, cx);
    }

    pub(crate) fn listener<Event: ?Sized>(
        &self,
        action: impl Fn(&Event, &mut Window, &mut App) -> Action + 'static,
    ) -> impl Fn(&Event, &mut Window, &mut App) + 'static {
        let sink = self.clone();
        move |event, window, cx| {
            let action = action(event, window, cx);
            sink.emit(action, window, cx);
        }
    }
}
