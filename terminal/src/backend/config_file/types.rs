use std::fmt::Debug;
use std::marker::PhantomData;

use serde::Deserialize;
use serde::Serialize;

pub trait ConfigTypes {
    type String: Serialize + for<'t> Deserialize<'t> + Debug + Default;
    type MaybeString: Serialize + for<'t> Deserialize<'t> + Debug + Default;
    type Port: Serialize + for<'t> Deserialize<'t> + Debug + Default;
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ConfigFileTypes<T = RuntimeTypes>(PhantomData<T>);

impl<T: ConfigTypes> ConfigTypes for ConfigFileTypes<T> {
    type String = Option<T::String>;
    type MaybeString = Option<T::MaybeString>;
    type Port = Option<T::Port>;
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RuntimeTypes(PhantomData<()>);

impl ConfigTypes for RuntimeTypes {
    type String = String;
    type MaybeString = Option<String>;
    type Port = u16;
}
