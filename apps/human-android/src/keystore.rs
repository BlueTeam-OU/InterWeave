// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The embedded custody path's platform cipher on the phone: custody's
//! `SeedCipher` over `KeystoreCipher.kt`, an AES-256-GCM key in
//! `AndroidKeyStore` per alias (android-key-custody.md). The Kotlin side
//! frames every answer -- a status byte, then the payload -- so no Java
//! exception reaches this side; the status maps one to one onto custody's
//! closed failure set.
//!
//! The seed's wiping is best-effort, as the seam says. Opening, the
//! Keystore hands the plaintext over as a Java `byte[]`, which this side
//! copies into a stack buffer and then overwrites with zeros; sealing, the
//! `byte[]` this side passes is cleared by `KeystoreCipher.seal` before it
//! returns. Either is the only wipe Java allows; what the collector
//! copied meanwhile is out of reach.

use interweave_human_android_platform::KeyUnlockPolicy;
use interweave_human_android_platform::custody::{
    CipherFailure, IV_LEN, KeyRef, SEALED_LEN, SEED_LEN, Sealed, Seed, SeedCipher,
};
use jni::objects::{JByteArray, JClass, JString, JValue, JValueOwned};
use jni::refs::Global;
use jni::{Env, JavaVM, jni_sig, jni_str};

/// `KeystoreCipher.kt`'s status bytes.
mod status {
    pub(super) const OK: u8 = 0;
    pub(super) const KEY_INVALIDATED: u8 = 1;
    pub(super) const KEY_MISSING: u8 = 2;
    pub(super) const USER_NOT_AUTHENTICATED: u8 = 3;
    pub(super) const AUTHENTICATION: u8 = 4;
}

/// The Keystore, reached from whatever thread custody calls on.
pub(crate) struct KeystoreCipher {
    vm: JavaVM,
    /// Found once on a thread the app's class loader serves (a native
    /// method's): a thread attached from native code would find only the
    /// system's classes.
    class: Global<JClass<'static>>,
}

impl KeystoreCipher {
    /// The cipher, built inside a native method of the app.
    ///
    /// # Errors
    /// The class or the VM could not be reached.
    pub(crate) fn new(env: &mut Env<'_>) -> jni::errors::Result<Self> {
        let class = env.find_class(jni_str!("org/interweave/human/KeystoreCipher"))?;
        Ok(Self {
            vm: env.get_java_vm()?,
            class: env.new_global_ref(class)?,
        })
    }

    /// Call `name` with `args`, answering its status byte and payload.
    fn call(
        &self,
        name: &'static jni::strings::JNIStr,
        sig: &jni::signature::MethodSignature<'static, 'static>,
        args: impl for<'a> FnOnce(&mut Env<'a>) -> jni::errors::Result<Vec<JValueOwned<'a>>>,
        payload: impl FnOnce(&mut Env<'_>, &JByteArray<'_>, usize) -> Result<Answer, CipherFailure>,
    ) -> Result<Answer, CipherFailure> {
        let unavailable = |e: jni::errors::Error| CipherFailure::Unavailable(format!("jni: {e}"));
        self.vm
            .attach_current_thread(
                |env| -> Result<Result<Answer, CipherFailure>, jni::errors::Error> {
                    let owned = args(env)?;
                    let values: Vec<JValue<'_>> = owned.iter().map(JValue::from).collect();
                    let answer = env
                        .call_static_method(&*self.class, name, sig, &values)?
                        .l()?;
                    let array = JByteArray::cast_local(env, answer)?;
                    let len = array.len(env)?;
                    let mut first = [0_i8; 1];
                    if len == 0 {
                        return Ok(Err(CipherFailure::Unavailable(
                            "an empty answer".to_owned(),
                        )));
                    }
                    array.get_region(env, 0, &mut first)?;
                    Ok(match first[0].cast_unsigned() {
                        status::OK => payload(env, &array, len),
                        code => Err(failure(code, &env.convert_byte_array(&array)?[1..])),
                    })
                },
            )
            .unwrap_or_else(|e| Err(unavailable(e)))
    }
}

/// What a successful call carried.
enum Answer {
    Sealed(Sealed),
    Seed(Seed),
    Done,
}

/// The failure a status byte names; `message` is for the log.
fn failure(code: u8, message: &[u8]) -> CipherFailure {
    match code {
        status::KEY_INVALIDATED => CipherFailure::KeyInvalidated,
        status::KEY_MISSING => CipherFailure::KeyMissing,
        status::USER_NOT_AUTHENTICATED => CipherFailure::UserNotAuthenticated,
        status::AUTHENTICATION => CipherFailure::Authentication,
        _ => CipherFailure::Unavailable(String::from_utf8_lossy(message).into_owned()),
    }
}

fn shape(what: &str) -> CipherFailure {
    CipherFailure::Unavailable(format!("the Keystore answered {what} of another length"))
}

impl SeedCipher for KeystoreCipher {
    fn seal(&self, key: &KeyRef, aad: &[u8], seed: &Seed) -> Result<Sealed, CipherFailure> {
        let alias = key.alias();
        let user_presence = key.policy() == KeyUnlockPolicy::UserPresence;
        let answer = self.call(
            jni_str!("seal"),
            &jni_sig!("(Ljava/lang/String;Z[B[B)[B"),
            |env| {
                Ok(vec![
                    JString::from_str(env, &alias)?.into(),
                    JValueOwned::Bool(user_presence),
                    env.byte_array_from_slice(aad)?.into(),
                    env.byte_array_from_slice(seed.expose())?.into(),
                ])
            },
            |env, array, len| {
                let bytes = env
                    .convert_byte_array(array)
                    .map_err(|e| CipherFailure::Unavailable(format!("jni: {e}")))?;
                if len != 1 + IV_LEN + SEALED_LEN {
                    return Err(shape("a seal"));
                }
                Ok(Answer::Sealed(Sealed {
                    iv: bytes[1..=IV_LEN].to_vec(),
                    sealed: bytes[1 + IV_LEN..].to_vec(),
                }))
            },
        );
        match answer? {
            Answer::Sealed(sealed) => Ok(sealed),
            _ => Err(shape("a seal")),
        }
    }

    fn retire(&self, key: &KeyRef) -> Result<(), CipherFailure> {
        let alias = key.alias();
        match self.call(
            jni_str!("retire"),
            &jni_sig!("(Ljava/lang/String;)[B"),
            |env| Ok(vec![JString::from_str(env, &alias)?.into()]),
            |_, _, _| Ok(Answer::Done),
        )? {
            Answer::Done => Ok(()),
            _ => Err(shape("a retire")),
        }
    }

    fn open(
        &self,
        key: &KeyRef,
        iv: &[u8; IV_LEN],
        sealed: &[u8; SEALED_LEN],
        aad: &[u8],
    ) -> Result<Seed, CipherFailure> {
        let alias = key.alias();
        match self.call(
            jni_str!("open"),
            &jni_sig!("(Ljava/lang/String;[B[B[B)[B"),
            |env| {
                Ok(vec![
                    JString::from_str(env, &alias)?.into(),
                    env.byte_array_from_slice(iv)?.into(),
                    env.byte_array_from_slice(sealed)?.into(),
                    env.byte_array_from_slice(aad)?.into(),
                ])
            },
            |env, array, len| {
                if len != 1 + SEED_LEN {
                    return Err(shape("a seed"));
                }
                let jni = |e: jni::errors::Error| CipherFailure::Unavailable(format!("jni: {e}"));
                let mut buf = [0_i8; SEED_LEN];
                array.get_region(env, 1, &mut buf).map_err(jni)?;
                let mut seed = [0_u8; SEED_LEN];
                for (to, from) in seed.iter_mut().zip(&mut buf) {
                    *to = from.cast_unsigned();
                    *from = 0;
                }
                // The Java copy, cleared as the seam's limit allows.
                array
                    .set_region(env, 0, &[0_i8; 1 + SEED_LEN])
                    .map_err(jni)?;
                Ok(Answer::Seed(Seed::new(seed)))
            },
        )? {
            Answer::Seed(seed) => Ok(seed),
            _ => Err(shape("a seed")),
        }
    }
}
