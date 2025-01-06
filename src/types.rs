use std::{
    any::Any,
    collections::HashMap,
    marker::PhantomData,
    ops::Deref,
    sync::{Arc, Mutex},
};

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

#[allow(dead_code)]
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

// impl std::fmt::Display for ListInterface {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         write!(f, "<ListInterface>")
//     }
// }

impl ListItemData for bool {}
impl ListItemData for f32 {}
impl ListItemData for f64 {}
impl ListItemData for i32 {}
impl ListItemData for i64 {}
impl ListItemData for u32 {}
impl ListItemData for u64 {}
impl ListItemData for String {}
// impl ListItemData for OpFnMut {}
// impl ListItemData for ListInterface<'_> {}

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
