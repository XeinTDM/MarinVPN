use dioxus::prelude::*;

#[component]
fn IconBase(
    size: u32, 
    #[props(default)] class: Option<String>, 
    #[props(default)] fill: Option<String>,
    #[props(default = 2)] stroke_width: u32,
    children: Element
) -> Element {
    let class = class.unwrap_or_default();
    let fill = fill.unwrap_or("none".to_string());
    rsx! {
        svg { 
            width: "{size}", 
            height: "{size}", 
            view_box: "0 0 24 24", 
            fill, 
            stroke: "currentColor", 
            stroke_width: "{stroke_width}", 
            stroke_linecap: "round", 
            stroke_linejoin: "round",
            class,
            {children}
        }
    }
}

#[component]
pub fn Shield(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            path { d: "M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z" }
        }
    }
}

#[component]
pub fn User(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            path { d: "M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2" }
            circle { cx: "12", cy: "7", r: "4" }
        }
    }
}

#[component]
pub fn Settings(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            path { d: "M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" }
            circle { cx: "12", cy: "12", r: "3" }
        }
    }
}

#[component]
pub fn Globe(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            circle { cx: "12", cy: "12", r: "10" }
            path { d: "M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" }
            path { d: "M2 12h20" }
        }
    }
}

#[component]
pub fn X(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            path { d: "M18 6 6 18" }
            path { d: "M6 6l12 12" }
        }
    }
}

#[component]
pub fn Minus(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            path { d: "M5 12h14" }
        }
    }
}

#[component]
pub fn ArrowLeft(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            path { d: "m12 19-7-7 7-7" }
            path { d: "M19 12H5" }
        }
    }
}

#[component]
pub fn ChevronRight(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            path { d: "m9 18 6-6-6-6" }
        }
    }
}

#[component]
pub fn ChevronDown(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            path { d: "m6 9 6 6 6-6" }
        }
    }
}

#[component]
pub fn Search(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            circle { cx: "11", cy: "11", r: "8" }
            path { d: "m21 21-4.3-4.3" }
        }
    }
}

#[component]
pub fn Star(size: u32, #[props(default)] class: Option<String>, #[props(default)] fill: Option<String>) -> Element {
    rsx! {
        IconBase { size, class, fill,
            path { d: "M11.525 2.223a.6.6 0 0 1 .95 0l2.31 3.407a.6.6 0 0 0 .434.24l4.105.442a.6.6 0 0 1 .332 1.02l-3.068 2.77a.6.6 0 0 0-.185.57l.84 4.052a.6.6 0 0 1-.87.632l-3.604-2.033a.6.6 0 0 0-.57 0l-3.604 2.033a.6.6 0 0 1-.87-.632l.84-4.052a.6.6 0 0 0-.185-.57l-3.068-2.77a.6.6 0 0 1 .332-1.02l4.105-.442a.6.6 0 0 0 .434-.24l2.31-3.407Z" }
        }
    }
}

#[component]
pub fn Loader(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            path { d: "M12 2v4" }
            path { d: "m16.2 7.8 2.9-2.9" }
            path { d: "M18 12h4" }
            path { d: "m16.2 16.2 2.9 2.9" }
            path { d: "M12 18v4" }
            path { d: "m4.9 19.1 2.9-2.9" }
            path { d: "M2 12h4" }
            path { d: "m4.9 4.9 2.9 2.9" }
        }
    }
}

#[component]
pub fn Zap(size: u32, #[props(default)] class: Option<String>, #[props(default)] fill: Option<String>) -> Element {
    rsx! {
        IconBase { size, class, fill,
            path { d: "M13 2 3 14h9l-1 8 10-12h-9l1-8z" }
        }
    }
}

#[component]
pub fn Lock(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            rect { width: "18", height: "11", x: "3", y: "11", rx: "2", ry: "2" }
            path { d: "M7 11V7a5 5 0 0 1 10 0v4" }
        }
    }
}

#[component]
pub fn Info(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            circle { cx: "12", cy: "12", r: "10" }
            path { d: "M12 16v-4" }
            path { d: "M12 8h.01" }
        }
    }
}

#[component]
pub fn CircleCheck(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            circle { cx: "12", cy: "12", r: "10" }
            path { d: "m9 12 2 2 4-4" }
        }
    }
}

#[component]
pub fn CircleAlert(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            circle { cx: "12", cy: "12", r: "10" }
            line { x1: "12", x2: "12", y1: "8", y2: "12" }
            line { x1: "12", x2: "12.01", y1: "16", y2: "16" }
        }
    }
}

#[component]
pub fn TriangleAlert(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            path { d: "m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z" }
            line { x1: "12", x2: "12", y1: "9", y2: "13" }
            line { x1: "12", x2: "12.01", y1: "17", y2: "17" }
        }
    }
}

#[component]
pub fn LifeBuoy(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            circle { cx: "12", cy: "12", r: "10" }
            circle { cx: "12", cy: "12", r: "4" }
            line { x1: "4.93", x2: "9.17", y1: "4.93", y2: "9.17" }
            line { x1: "14.83", x2: "19.07", y1: "14.83", y2: "19.07" }
            line { x1: "14.83", x2: "19.07", y1: "4.93", y2: "9.17" }
            line { x1: "4.93", x2: "9.17", y1: "14.83", y2: "19.07" }
        }
    }
}

#[component]
pub fn BookOpen(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            path { d: "M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z" }
            path { d: "M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z" }
        }
    }
}

#[component]
pub fn MessageCircle(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            path { d: "M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z" }
        }
    }
}

#[component]
pub fn ShieldCheck(size: u32, #[props(default)] class: Option<String>, #[props(default = 2)] stroke_width: u32) -> Element {
    rsx! {
        IconBase { size, class, stroke_width,
            path { d: "M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z" }
            path { d: "m9 12 2 2 4-4" }
        }
    }
}

#[component]
pub fn ArrowRight(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            path { d: "M5 12h14" }
            path { d: "m12 5 7 7-7 7" }
        }
    }
}

#[component]
pub fn RefreshCw(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            path { d: "M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8" }
            path { d: "M21 3v5h-5" }
            path { d: "M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16" }
            path { d: "M3 21v-5h5" }
        }
    }
}

#[component]
pub fn FlaskConical(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            path { d: "M10 2v7.527a2 2 0 0 1-.211.896L4.72 20.55a1 1 0 0 0 .9 1.45h12.76a1 1 0 0 0 .9-1.45l-5.069-10.127A2 2 0 0 1 14 9.527V2" }
            path { d: "M8.5 2h7" }
            path { d: "M7 16h10" }
        }
    }
}

#[component]
pub fn ArrowDown(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            path { d: "M12 5v14" }
            path { d: "m19 12-7 7-7-7" }
        }
    }
}

#[component]
pub fn ArrowUp(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            path { d: "M12 19V5" }
            path { d: "m5 12 7-7 7 7" }
        }
    }
}

#[component]
pub fn Clock(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            circle { cx: "12", cy: "12", r: "10" }
            polyline { points: "12 6 12 12 16 14" }
        }
    }
}

#[component]
pub fn Check(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            path { d: "M20 6 9 17l-5-5" }
        }
    }
}

#[component]
pub fn LogOut(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            path { d: "M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" }
            polyline { points: "16 17 21 12 16 7" }
            line { x1: "21", x2: "9", y1: "12", y2: "12" }
        }
    }
}

#[component]
pub fn Eye(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            path { d: "M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z" }
            circle { cx: "12", cy: "12", r: "3" }
        }
    }
}

#[component]
pub fn Copy(size: u32, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        IconBase { size, class,
            rect { width: "14", height: "14", x: "8", y: "8", rx: "2", ry: "2" }
            path { d: "M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2" }
        }
    }
}