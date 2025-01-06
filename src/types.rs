use std::{
    any::Any,
    collections::HashMap,
    marker::PhantomData,
    ops::Deref,
    sync::{Arc, Mutex},
};

use gilrs::{Axis, Button};

pub trait ListItemData: 'static + Send + Sync + ToAny + std::fmt::Display {}

pub struct ValueStore {
    pub map: HashMap<String, Box<dyn ListItemData>>,
}

impl ValueStore {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Value<dyn ListItemData> {
        Value {
            p: PhantomData,
            key: key.to_string(),
        }
    }

    pub fn insert<T: 'static + ListItemData>(
        &mut self,
        key: &str,
        v: T,
    ) -> Arc<Mutex<Value<dyn ListItemData>>> {
        Arc::new(Mutex::new(Value::<dyn ListItemData>::new(
            key,
            Box::new(v),
            self,
        )))
    }
}

// #[allow(dead_code)]
pub trait ToAny: 'static {
    fn as_any(&self) -> &dyn Any;
}

impl<T: 'static> ToAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[allow(dead_code)]
pub enum OperatorResult {
    Done,
    Cancelled,
    Irrelevant,
}

#[allow(dead_code)]
pub struct OpFnMut {
    callback: dyn FnMut(OperatorResult),
}

impl std::fmt::Display for OpFnMut {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "<OpFnMut>")
    }
}

impl ListItemData for bool {}
impl ListItemData for f32 {}
impl ListItemData for f64 {}
impl ListItemData for i32 {}
impl ListItemData for i64 {}
impl ListItemData for u32 {}
impl ListItemData for u64 {}
impl ListItemData for String {}
// impl ListItemData for OpFnMut {}

#[derive(Debug)]
pub struct Value<T>
where
    T: ListItemData + ?Sized,
{
    p: PhantomData<T>,
    pub key: String,
}

impl<T: ?Sized + 'static> Value<T>
where
    T: 'static + ListItemData,
{
    pub fn load<'a>(&self, store: &'a ValueStore) -> &'a dyn ListItemData {
        store.map.get(&self.key).unwrap().deref()
    }

    pub fn new(
        key: &str,
        boxed_value: Box<dyn ListItemData>,
        store: &mut ValueStore,
    ) -> Value<dyn ListItemData> {
        store.map.insert(key.to_string(), boxed_value);

        Value {
            p: PhantomData,
            key: key.to_string(),
        }
    }

    pub fn replace(&mut self, boxed_value: Box<dyn ListItemData>, store: &mut ValueStore) {
        store.map.remove(&self.key);
        store.map.insert(self.key.as_str().to_string(), boxed_value);
        self.p = PhantomData;
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum CommonAnalog {
    LeftStickX,
    LeftStickY,
    LeftLever,
    RightStickX,
    RightStickY,
    RightLever,
    DPadX,
    DPadY,
    Unknown,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum CommonButton {
    LeftStickPress,
    RightStickPress,
    LeftShoulder,
    RightShoulder,
    FaceSouth,
    FaceEast,
    FaceWest,
    FaceNorth,
    DPadSouth,
    DPadEast,
    DPadWest,
    DPadNorth,
    Start,
    Select,
    Guide,
    LegacyC,
    LegacyZ,
    LegacyLT,
    LegacyLT2,
    LegacyRT,
    LegacyRT2,
    Unknown,
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub struct StateAnalog {
    pub ty: CommonAnalog,
    pub value: f32,
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub struct StateButton {
    pub ty: CommonButton,
    pub value: f32,
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub struct BasicControllerState {
    pub analogs: [StateAnalog; 9],
    pub buttons: [StateButton; 22],
}

impl BasicControllerState {
    pub fn button_state_by_type(&self, by: CommonButton) -> StateButton {
        match by {
            CommonButton::LeftStickPress => self.buttons[0],
            CommonButton::RightStickPress => self.buttons[1],
            CommonButton::LeftShoulder => self.buttons[2],
            CommonButton::RightShoulder => self.buttons[3],
            CommonButton::FaceSouth => self.buttons[4],
            CommonButton::FaceEast => self.buttons[5],
            CommonButton::FaceWest => self.buttons[6],
            CommonButton::FaceNorth => self.buttons[7],
            CommonButton::DPadSouth => self.buttons[8],
            CommonButton::DPadEast => self.buttons[9],
            CommonButton::DPadWest => self.buttons[10],
            CommonButton::DPadNorth => self.buttons[11],
            CommonButton::Start => self.buttons[12],
            CommonButton::Select => self.buttons[13],
            CommonButton::Guide => self.buttons[14],
            CommonButton::LegacyC => self.buttons[15],
            CommonButton::LegacyZ => self.buttons[16],
            CommonButton::LegacyLT => self.buttons[17],
            CommonButton::LegacyLT2 => self.buttons[18],
            CommonButton::LegacyRT => self.buttons[19],
            CommonButton::LegacyRT2 => self.buttons[20],
            CommonButton::Unknown => self.buttons[21],
        }
    }

    pub fn analog_state_by_type(&self, by: CommonAnalog) -> StateAnalog {
        match by {
            CommonAnalog::LeftStickX => self.analogs[0],
            CommonAnalog::LeftStickY => self.analogs[1],
            CommonAnalog::LeftLever => self.analogs[2],
            CommonAnalog::RightStickX => self.analogs[3],
            CommonAnalog::RightStickY => self.analogs[4],
            CommonAnalog::RightLever => self.analogs[5],
            CommonAnalog::DPadX => self.analogs[6],
            CommonAnalog::DPadY => self.analogs[7],
            CommonAnalog::Unknown => self.analogs[8],
        }
    }

    pub fn try_update_button(&mut self, ty: CommonButton, value: f32) {
        for button in self.buttons.iter_mut() {
            if button.ty == ty {
                button.value = value;
            }
        }
    }

    pub fn try_update_analog(&mut self, ty: CommonAnalog, value: f32) {
        for analog in self.analogs.iter_mut() {
            if analog.ty == ty {
                analog.value = value;
            }
        }
    }
}

impl Default for BasicControllerState {
    fn default() -> Self {
        Self {
            analogs: [
                StateAnalog {
                    ty: CommonAnalog::LeftStickX,
                    value: 0.0,
                },
                StateAnalog {
                    ty: CommonAnalog::LeftStickY,
                    value: 0.0,
                },
                StateAnalog {
                    ty: CommonAnalog::LeftLever,
                    value: 0.0,
                },
                StateAnalog {
                    ty: CommonAnalog::RightStickX,
                    value: 0.0,
                },
                StateAnalog {
                    ty: CommonAnalog::RightStickY,
                    value: 0.0,
                },
                StateAnalog {
                    ty: CommonAnalog::RightLever,
                    value: 0.0,
                },
                StateAnalog {
                    ty: CommonAnalog::DPadX,
                    value: 0.0,
                },
                StateAnalog {
                    ty: CommonAnalog::DPadY,
                    value: 0.0,
                },
                StateAnalog {
                    ty: CommonAnalog::Unknown,
                    value: 0.0,
                },
            ],
            buttons: [
                StateButton {
                    ty: CommonButton::LeftStickPress,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::RightStickPress,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::LeftShoulder,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::RightShoulder,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::FaceSouth,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::FaceEast,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::FaceWest,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::FaceNorth,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::DPadSouth,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::DPadEast,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::DPadWest,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::DPadNorth,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::Start,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::Select,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::Guide,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::LegacyC,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::LegacyZ,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::LegacyLT,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::LegacyLT2,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::LegacyRT,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::LegacyRT2,
                    value: 0.0,
                },
                StateButton {
                    ty: CommonButton::Unknown,
                    value: 0.0,
                },
            ],
        }
    }
}

pub fn button_to_bcs(button: gilrs::Button) -> CommonButton {
    match button {
        Button::South => CommonButton::FaceSouth,
        Button::East => CommonButton::FaceEast,
        Button::North => CommonButton::FaceNorth,
        Button::West => CommonButton::FaceWest,
        Button::Select => CommonButton::Select,
        Button::Start => CommonButton::Start,
        Button::Mode => CommonButton::Guide,
        Button::LeftThumb => CommonButton::LeftStickPress,
        Button::RightThumb => CommonButton::RightStickPress,
        Button::DPadUp => CommonButton::DPadNorth,
        Button::DPadDown => CommonButton::DPadSouth,
        Button::DPadLeft => CommonButton::DPadWest,
        Button::DPadRight => CommonButton::DPadEast,
        Button::C => CommonButton::LegacyC,
        Button::Z => CommonButton::LegacyZ,
        Button::LeftTrigger => CommonButton::LegacyLT,
        Button::LeftTrigger2 => CommonButton::LegacyLT2,
        Button::RightTrigger => CommonButton::LegacyRT,
        Button::RightTrigger2 => CommonButton::LegacyRT2,
        Button::Unknown => CommonButton::Unknown,
    }
}

pub fn axis_to_bcs(axis: gilrs::Axis) -> CommonAnalog {
    match axis {
        Axis::LeftStickX => CommonAnalog::LeftStickX,
        Axis::LeftStickY => CommonAnalog::LeftStickY,
        Axis::LeftZ => CommonAnalog::LeftLever,
        Axis::RightStickX => CommonAnalog::RightStickX,
        Axis::RightStickY => CommonAnalog::RightStickY,
        Axis::RightZ => CommonAnalog::RightLever,
        Axis::DPadX => CommonAnalog::DPadX,
        Axis::DPadY => CommonAnalog::DPadY,
        Axis::Unknown => CommonAnalog::Unknown,
    }
}
