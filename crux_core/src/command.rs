use bcs::from_bytes;
use serde::de::DeserializeOwned;

/// Command captures the intent for a side-effect. Commands are return by the [`App::update`] function.
///
/// You should never create a Command yourself, instead use one of the capabilities to create a command.
/// Command is generic over `Message` in order to carry a "callback" which will be sent to the [`App::update`]
/// function when the command has been executed, and passed the resulting data.
pub struct Command<Ef, Ev> {
    pub(crate) effect: Ef, // TODO switch to `enum Effect`, so that shell knows what to do
    pub(crate) resolve: Option<Box<dyn Callback<Ev> + Send + Sync>>,
}

impl<Ef, Ev> Command<Ef, Ev> {
    pub fn new<F, T>(effect: Ef, resolve: F) -> Self
    where
        F: Fn(T) -> Ev + Send + Sync + 'static,
        Ev: 'static,
        T: 'static + DeserializeOwned,
    {
        Self {
            effect,
            resolve: Some(Box::new(resolve.into_callback())),
        }
    }

    pub fn new_without_callback(effect: Ef) -> Self {
        Self {
            effect,
            resolve: None,
        }
    }

    pub fn resolve(&self, value: Vec<u8>) -> Ev {
        if let Some(resolve) = &self.resolve {
            return resolve.call(value);
        }

        panic!("mismatched capability response");
    }

    /// Lift is used to convert a Command with one message type to a command with another.
    ///
    /// This is normally used when composing applications. A typical case in the top-level
    /// `update` function would look like the following:
    ///
    /// ```rust,ignore
    /// match message {
    ///     // ...
    ///     Msg::Submodule(msg) => Command::lift(
    ///             self.submodule.update(msg, &mut model.submodule),
    ///             Msg::Submodule,
    ///         ),
    ///     // ...
    /// }
    /// ```
    pub fn lift<ParentEv, F>(commands: Vec<Command<Ef, Ev>>, f: F) -> Vec<Command<Ef, ParentEv>>
    where
        F: Fn(Ev) -> ParentEv + Sync + Send + Copy + 'static,
        Ev: 'static,
        ParentEv: 'static,
    {
        commands.into_iter().map(move |c| c.map(f)).collect()
    }

    fn map<ParentEvent, F>(self, f: F) -> Command<Ef, ParentEvent>
    where
        F: Fn(Ev) -> ParentEvent + Sync + Send + Copy + 'static,
        Ev: 'static,
        ParentEvent: 'static,
    {
        Command {
            effect: self.effect,
            resolve: match self.resolve {
                Some(resolve) => {
                    let callback = move |capability_response: Vec<u8>| {
                        // FIXME: remove the need for this (by avoiding double deserialization)
                        let response = bcs::to_bytes(&capability_response).unwrap();

                        f(resolve.call(response))
                    };
                    Some(Box::new(callback.into_callback()))
                }
                None => None,
            },
        }
    }
}

pub trait Callback<Event> {
    fn call(&self, value: Vec<u8>) -> Event;
}

struct CallBackFn<T, Event> {
    function: Box<dyn Fn(T) -> Event + Send + Sync>,
}

impl<T, Event> Callback<Event> for CallBackFn<T, Event>
where
    T: DeserializeOwned,
{
    fn call(&self, value: Vec<u8>) -> Event {
        let response = from_bytes::<T>(&value).unwrap();
        (self.function)(response)
    }
}

trait IntoCallBack<T, Event> {
    fn into_callback(self) -> CallBackFn<T, Event>;
}

impl<F, T, Event> IntoCallBack<T, Event> for F
where
    F: Fn(T) -> Event + Send + Sync + 'static,
{
    fn into_callback(self) -> CallBackFn<T, Event> {
        CallBackFn {
            function: Box::new(self),
        }
    }
}