#![allow(clippy::missing_safety_doc)]

use jni::{
  objects::{JClass, JObject, JObjectArray, JString, JValueGen},
  sys::{jboolean, jchar, jint, jlong, jobject, jstring},
  JNIEnv,
};
use std::mem::transmute;

const MATCH_CLASS: &str = "com/github/null8626/decancer/Match";

macro_rules! jni_unwrap {
  ($env:ident, $value:expr, $return_value:expr) => {
    match $value {
      Ok(output) => output,
      Err(error) => {
        let _ = $env.throw_new("java/lang/RuntimeException", error.to_string());

        return $return_value;
      }
    }
  };

  ($env:ident, $value:expr) => {
    jni_unwrap!($env, $value, 0 as _)
  };
}

macro_rules! get_inner_field {
  ($env:ident, $this:ident, $return_value:expr) => {{
    let inner = jni_unwrap!(
      $env,
      $env
        .get_field(&$this, "inner", "J")
        .and_then(|field| field.j()),
      $return_value
    ) as *mut decancer::CuredString;

    if inner.is_null() {
      let _ = $env.throw_new(
        "java/lang/NullPointerException",
        "destroy() has been called prior to this.",
      );

      return $return_value;
    }

    inner
  }};

  ($env:ident, $this:ident) => {
    get_inner_field!($env, $this, 0 as _)
  };
}

macro_rules! get_string_array {
  ($env:ident, $input:ident, $return_value:expr) => {{
    let input_len = jni_unwrap!($env, $env.get_array_length(&$input), $return_value);
    let mut inputs: Vec<String> = Vec::with_capacity(input_len as _);

    for i in 0..input_len {
      let obj = $env.get_object_array_element(&$input, i);

      inputs.push(
        jni_unwrap!(
          $env,
          $env.get_string(&JString::from(jni_unwrap!($env, obj, $return_value))),
          $return_value
        )
        .into(),
      );
    }

    inputs
  }};

  ($env:ident, $input:ident) => {
    get_string_array!($env, $input, 0 as _)
  };
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_github_null8626_decancer_CuredString_cure<'local>(
  mut env: JNIEnv<'local>,
  _: JClass<'local>,
  input: JString<'local>,
  options: jint,
) -> jlong {
  let input: String = jni_unwrap!(env, env.get_string(&input)).into();

  match decancer::cure(&input, transmute(options)) {
    Ok(output) => Box::into_raw(Box::new(output)) as _,

    Err(error) => {
      let _ = env.throw_new(
        "java/lang/IllegalArgumentException",
        <decancer::Error as AsRef<str>>::as_ref(&error),
      );

      0 as _
    }
  }
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_github_null8626_decancer_CuredString_find<'local>(
  mut env: JNIEnv<'local>,
  this: JObject<'local>,
  input: JString<'local>,
) -> jobject {
  let inner = get_inner_field!(env, this);
  let input: String = jni_unwrap!(env, env.get_string(&input)).into();

  let matches = (*inner).find(&input).collect::<Vec<_>>();
  let array = jni_unwrap!(
    env,
    env.new_object_array(matches.len() as _, MATCH_CLASS, JObject::null())
  );

  for (idx, result) in matches.into_iter().enumerate() {
    let element = jni_unwrap!(
      env,
      env.new_object(
        MATCH_CLASS,
        "(JJLjava/lang/String;)V",
        &[
          JValueGen::Long(result.start as _),
          JValueGen::Long(result.end as _),
          JValueGen::Object(
            &jni_unwrap!(
              env,
              env.new_string(unsafe { (*inner).get_unchecked(result) })
            )
            .into()
          ),
        ]
      )
    );

    jni_unwrap!(env, env.set_object_array_element(&array, idx as _, element));
  }

  array.into_raw()
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_github_null8626_decancer_CuredString_findMultiple<'local>(
  mut env: JNIEnv<'local>,
  this: JObject<'local>,
  input: JObjectArray<'local>,
) -> jobject {
  let inner = get_inner_field!(env, this);
  let inputs = get_string_array!(env, input);

  let matches = (*inner).find_multiple(&inputs);
  let array = jni_unwrap!(
    env,
    env.new_object_array(matches.len() as _, MATCH_CLASS, JObject::null())
  );

  for (idx, result) in matches.into_iter().enumerate() {
    let element = jni_unwrap!(
      env,
      env.new_object(
        MATCH_CLASS,
        "(JJLjava/lang/String;)V",
        &[
          JValueGen::Long(result.start as _),
          JValueGen::Long(result.end as _),
          JValueGen::Object(
            &jni_unwrap!(
              env,
              env.new_string(unsafe { (*inner).get_unchecked(result) })
            )
            .into()
          ),
        ]
      )
    );

    jni_unwrap!(env, env.set_object_array_element(&array, idx as _, element));
  }

  array.into_raw()
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_github_null8626_decancer_CuredString_censor<'local>(
  mut env: JNIEnv<'local>,
  this: JObject<'local>,
  input: JString<'local>,
  with: jchar,
) {
  let inner = get_inner_field!(env, this, ());
  let input: String = jni_unwrap!(env, env.get_string(&input), ()).into();

  match char::from_u32(with as _) {
    Some(with) => {
      (*inner).censor(&input, with);
    }

    None => {
      let _ = env.throw_new(
        "java/lang/IllegalArgumentException",
        "Replacement character is a surrogate.",
      );
    }
  };
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_github_null8626_decancer_CuredString_censorMultiple<
  'local,
>(
  mut env: JNIEnv<'local>,
  this: JObject<'local>,
  input: JObjectArray<'local>,
  with: jchar,
) {
  let inner = get_inner_field!(env, this, ());
  let inputs = get_string_array!(env, input, ());

  match char::from_u32(with as _) {
    Some(with) => {
      (*inner).censor_multiple(&inputs, with);
    }

    None => {
      let _ = env.throw_new(
        "java/lang/IllegalArgumentException",
        "Replacement character is a surrogate.",
      );
    }
  };
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_github_null8626_decancer_CuredString_replace<'local>(
  mut env: JNIEnv<'local>,
  this: JObject<'local>,
  input: JString<'local>,
  with: JString<'local>,
) {
  let inner = get_inner_field!(env, this, ());
  let input: String = jni_unwrap!(env, env.get_string(&input), ()).into();
  let with: String = jni_unwrap!(env, env.get_string(&with), ()).into();

  (*inner).replace(&input, &with);
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_github_null8626_decancer_CuredString_replaceMultiple<
  'local,
>(
  mut env: JNIEnv<'local>,
  this: JObject<'local>,
  input: JObjectArray<'local>,
  with: JString<'local>,
) {
  let inner = get_inner_field!(env, this, ());
  let inputs = get_string_array!(env, input, ());
  let with: String = jni_unwrap!(env, env.get_string(&with), ()).into();

  (*inner).replace_multiple(&inputs, &with);
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_github_null8626_decancer_CuredString_equals<'local>(
  mut env: JNIEnv<'local>,
  this: JObject<'local>,
  input: JString<'local>,
) -> jboolean {
  let inner = get_inner_field!(env, this);
  let input: String = jni_unwrap!(env, env.get_string(&input)).into();

  transmute((*inner) == input)
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_github_null8626_decancer_CuredString_startsWith<'local>(
  mut env: JNIEnv<'local>,
  this: JObject<'local>,
  input: JString<'local>,
) -> jboolean {
  let inner = get_inner_field!(env, this);
  let input: String = jni_unwrap!(env, env.get_string(&input)).into();

  transmute((*inner).starts_with(&input))
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_github_null8626_decancer_CuredString_endsWith<'local>(
  mut env: JNIEnv<'local>,
  this: JObject<'local>,
  input: JString<'local>,
) -> jboolean {
  let inner = get_inner_field!(env, this);
  let input: String = jni_unwrap!(env, env.get_string(&input)).into();

  transmute((*inner).ends_with(&input))
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_github_null8626_decancer_CuredString_contains<'local>(
  mut env: JNIEnv<'local>,
  this: JObject<'local>,
  input: JString<'local>,
) -> jboolean {
  let inner = get_inner_field!(env, this);
  let input: String = jni_unwrap!(env, env.get_string(&input)).into();

  transmute((*inner).contains(&input))
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_github_null8626_decancer_CuredString_toString<'local>(
  mut env: JNIEnv<'local>,
  this: JObject<'local>,
) -> jstring {
  let inner = get_inner_field!(env, this);

  jni_unwrap!(env, env.new_string((*inner).as_str())).into_raw()
}

#[no_mangle]
#[allow(clippy::unused_unit)]
pub unsafe extern "system" fn Java_com_github_null8626_decancer_CuredString_destroy<'local>(
  mut env: JNIEnv<'local>,
  this: JObject<'local>,
) {
  let _ = Box::from_raw(get_inner_field!(env, this, ()));
}
