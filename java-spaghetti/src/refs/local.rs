use std::fmt::{self, Debug, Display, Formatter};
use std::ops::Deref;

use jni_sys::*;

use crate::{AssignableTo, Env, Global, JavaDebug, JavaDisplay, Ref, ReferenceType, Return};

/// A [Local](https://www.ibm.com/docs/en/sdk-java-technology/8?topic=collector-overview-jni-object-references),
/// non-null, reference to a Java object (+ [Env]) limited to the current thread/stack.
///
/// Including the env allows for the convenient execution of methods without having to individually pass the env as an
/// argument to each and every one.  Since this is limited to the current thread/stack, these cannot be sanely stored
/// in any kind of static storage, nor shared between threads - instead use a [Global] if you need to do either.
///
/// Will `DeleteLocalRef` when dropped, invalidating the jobject but ensuring threads that rarely or never return to
/// Java may run without being guaranteed to eventually exhaust their local reference limit.  If this is not desired,
/// convert to a plain Ref with:
///
/// ```rust,no_run
/// # use java_spaghetti::*;
/// # fn example<T: ReferenceType>(local: Local<T>) {
/// let local = Local::leak(local);
/// # }
/// ```
///
/// **Not FFI Safe:**  #\[repr(rust)\], and exact layout is likely to change - depending on exact features used - in the
/// future.  Specifically, on Android, since we're guaranteed to only have a single ambient VM, we can likely store the
/// \*const JNIEnv in thread local storage instead of lugging it around in every Local.  Of course, there's no
/// guarantee that's actually an *optimization*...
pub struct Local<'env, T: ReferenceType> {
    ref_: Ref<'env, T>,
}

impl<'env, T: ReferenceType> Local<'env, T> {
    pub unsafe fn from_raw(env: Env<'env>, object: jobject) -> Self {
        Self {
            ref_: Ref::from_raw(env, object),
        }
    }

    pub fn env(&self) -> Env<'env> {
        self.ref_.env()
    }

    pub fn as_raw(&self) -> jobject {
        self.ref_.as_raw()
    }

    pub fn into_raw(self) -> jobject {
        let object = self.ref_.as_raw();
        std::mem::forget(self); // Don't allow local to DeleteLocalRef the jobject
        object
    }

    pub fn leak(self) -> Ref<'env, T> {
        unsafe { Ref::from_raw(self.env(), self.into_raw()) }
    }

    pub fn as_global(&self) -> Global<T> {
        let env = self.env();
        let jnienv = env.as_raw();
        let object = unsafe { ((**jnienv).v1_2.NewGlobalRef)(jnienv, self.as_raw()) };
        assert!(!object.is_null());
        unsafe { Global::from_raw(env.vm(), object) }
    }

    pub fn as_ref(&self) -> &Ref<'_, T> {
        &self.ref_
    }

    pub fn as_return(&self) -> Return<'env, T> {
        let env: *mut *const JNINativeInterface_ = self.env().as_raw();
        let object = unsafe { ((**env).v1_2.NewLocalRef)(env, self.as_raw()) };
        assert!(!object.is_null());
        unsafe { Return::from_raw(object) }
    }

    pub fn into_return(self) -> Return<'env, T> {
        unsafe { Return::from_raw(self.into_raw()) }
    }

    pub fn cast<U: ReferenceType>(&self) -> Result<Local<'env, U>, crate::CastError> {
        let env = self.env();
        let jnienv = env.as_raw();
        let class1 = unsafe { ((**jnienv).v1_2.GetObjectClass)(jnienv, self.as_raw()) };
        let class2 = U::static_with_jni_type(|t| unsafe { env.require_class(t) });
        if !unsafe { ((**jnienv).v1_2.IsAssignableFrom)(jnienv, class1, class2) } {
            return Err(crate::CastError);
        }
        let object = unsafe { ((**jnienv).v1_2.NewLocalRef)(jnienv, self.as_raw()) };
        assert!(!object.is_null());
        Ok(unsafe { Local::from_raw(env, object) })
    }

    pub fn upcast<U: ReferenceType>(&self) -> Local<'env, U>
    where
        Self: AssignableTo<U>,
    {
        let env = self.env();
        let jnienv = env.as_raw();
        let object = unsafe { ((**jnienv).v1_2.NewLocalRef)(jnienv, self.as_raw()) };
        assert!(!object.is_null());
        unsafe { Local::from_raw(env, object) }
    }
}

impl<'env, T: ReferenceType> From<Ref<'env, T>> for Local<'env, T> {
    fn from(x: Ref<'env, T>) -> Self {
        x.as_local()
    }
}

impl<'env, T: ReferenceType> From<&Local<'env, T>> for Local<'env, T> {
    fn from(x: &Local<'env, T>) -> Self {
        x.clone()
    }
}

impl<'env, T: ReferenceType> From<&Ref<'env, T>> for Local<'env, T> {
    fn from(x: &Ref<'env, T>) -> Self {
        x.as_local()
    }
}

impl<'env, T: ReferenceType> Deref for Local<'env, T> {
    type Target = Ref<'env, T>;
    fn deref(&self) -> &Self::Target {
        &self.ref_
    }
}

impl<'env, T: ReferenceType> Clone for Local<'env, T> {
    fn clone(&self) -> Self {
        let env = self.env().as_raw();
        let object = unsafe { ((**env).v1_2.NewLocalRef)(env, self.as_raw()) };
        assert!(!object.is_null());
        unsafe { Self::from_raw(self.env(), object) }
    }
}

impl<'env, T: ReferenceType> Drop for Local<'env, T> {
    fn drop(&mut self) {
        let env = self.env().as_raw();
        unsafe { ((**env).v1_2.DeleteLocalRef)(env, self.as_raw()) }
    }
}

impl<'env, T: JavaDebug> Debug for Local<'env, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        T::fmt(self, f)
    }
}

impl<'env, T: JavaDisplay> Display for Local<'env, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        T::fmt(self, f)
    }
}
