use gpui::*;
use smallvec::SmallVec;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

pub struct ContextMenu {
    pub(self) id: ElementId,
    pub(self) style: StyleRefinement,
    pub(self) element: Option<AnyElement>,
    pub(self) menu: Option<Div>,
}

impl ContextMenu {
    pub fn with(mut self, element: impl IntoElement) -> Self {
        self.element = Some(element.into_any_element());
        self
    }
}

impl Styled for ContextMenu {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl IntoElement for ContextMenu {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl ParentElement for ContextMenu {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.menu.as_mut().unwrap().extend(elements);
    }
}

struct ContextMenuState {
    pub clicked_in: AtomicBool,
    pub position: RefCell<Option<Point<Pixels>>>,
}

impl ContextMenuState {
    pub fn new() -> Self {
        ContextMenuState {
            clicked_in: AtomicBool::new(false),
            position: RefCell::new(None),
        }
    }
}

impl Element for ContextMenu {
    type RequestLayoutState = Option<(Anchored, LayoutId, AnchoredState)>;

    type PrepaintState = Option<Bounds<Pixels>>;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        cx: &mut WindowContext,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style::default();

        let mut layout_ids: SmallVec<[LayoutId; 2]> = SmallVec::new();

        if let Some(element) = self.element.as_mut() {
            layout_ids.push(element.request_layout(cx));
        }

        let menu = self.menu.take();

        let anchored =
            cx.with_element_state(id.unwrap(), move |prev: Option<Rc<ContextMenuState>>, _| {
                let state = prev.unwrap_or_else(|| Rc::new(ContextMenuState::new()));

                let point = state.position.borrow().clone();

                if let (Some(position), Some(menu)) = (point, menu) {
                    let state_clone = state.clone();

                    let new = anchored().position(position).child(deferred(
                        menu.occlude().on_mouse_down_out(move |_, _| {
                            (*state_clone.position.borrow_mut()) = None;
                        }),
                    ));
                    (Some(new), state)
                } else {
                    (None, state)
                }
            });

        let state = if let Some(mut anchored) = anchored {
            let layout = anchored.request_layout(None, cx);
            layout_ids.push(layout.0);
            Some((anchored, layout.0, layout.1))
        } else {
            None
        };

        (cx.request_layout(style, layout_ids), state)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        cx: &mut WindowContext,
    ) -> Self::PrepaintState {
        if let Some(element) = self.element.as_mut() {
            element.prepaint(cx);
        }

        if let Some(anchored) = request_layout {
            let bounds = cx.layout_bounds(anchored.1);

            anchored
                .0
                .prepaint(None, cx.layout_bounds(anchored.1), &mut anchored.2, cx);

            Some(bounds)
        } else {
            None
        }
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        cx: &mut WindowContext,
    ) {
        if let Some(element) = self.element.as_mut() {
            element.paint(cx);
        }

        if let Some(anchored) = request_layout {
            anchored
                .0
                .paint(None, prepaint.unwrap(), &mut anchored.2, &mut (), cx);
        }

        cx.with_element_state(id.unwrap(), |prev: Option<Rc<ContextMenuState>>, cx| {
            let state = prev.unwrap_or_else(|| Rc::new(ContextMenuState::new()));
            let state_clone = state.clone();

            cx.on_mouse_event(move |ev: &MouseDownEvent, phase, _| {
                if ev.button == MouseButton::Right
                    && phase == DispatchPhase::Bubble
                    && bounds.contains(&ev.position)
                {
                    state_clone.clicked_in.store(true, Ordering::Release);
                }
            });

            let state_clone_2 = state.clone();

            cx.on_mouse_event(move |ev: &MouseUpEvent, phase, _| {
                if phase == DispatchPhase::Bubble {
                    let clicked_in = state_clone_2.clicked_in.swap(false, Ordering::AcqRel);

                    if ev.button == MouseButton::Right
                        && bounds.contains(&ev.position)
                        && clicked_in
                    {
                        (*state_clone_2.position.borrow_mut()) = Some(ev.position)
                    }
                }
            });

            ((), state)
        })
    }
}

pub fn context(id: impl Into<ElementId>) -> ContextMenu {
    ContextMenu {
        id: id.into(),
        style: StyleRefinement::default(),
        element: None,
        menu: Some(div()),
    }
}
