#![doc = include_str!("../README.md")]

use ctor::ctor;
use jni::objects::{JMethodID, JObject, JValue};
use jni::signature::{Primitive, ReturnType};
use jni::sys::jvalue;
use jni::{InitArgsBuilder, JNIEnv, JNIVersion, JavaVM};
use std::io::Read;
use std::path::Path;

#[ctor]
static JVM: JavaVM = {
    let jvm_args = InitArgsBuilder::new()
        .version(JNIVersion::V8)
        //.option("-Xcheck:jni")
        .build()
        .unwrap();

    let jvm = JavaVM::new(jvm_args).unwrap();
    jvm
};

pub struct Generator<'a> {
    generator_java_instance: JObject<'a>,
    get_block_method: JMethodID,
}

impl<'a> Generator<'a> {
    fn load_jar(env: &mut JNIEnv, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let file = std::fs::File::open(path)?;

        let mut archive = zip::ZipArchive::new(file)?;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let file_name = file.name();
            if !file_name.ends_with(".class") {
                continue;
            }

            let java_path = file_name.replace(".class", "");
            println!("loading: {}", java_path);

            let mut class_data = Vec::new();
            file.read_to_end(&mut class_data)?;
            env.define_class(&java_path, &JObject::null(), &class_data)?;
        }
        Ok(())
    }

    pub fn new(path: impl AsRef<Path>, seed: i64) -> anyhow::Result<Self> {
        JVM.attach_current_thread_as_daemon().unwrap();

        let mut env = JVM.get_env()?;

        Self::load_jar(&mut env, path)?;

        let generator_class = env.find_class("org/archipel/generator/Generator")?;
        let jvalue = JValue::from(seed);
        let generator_java_instance = env.new_object(&generator_class, "(J)V", &[jvalue])?;
        let get_block_method = env.get_method_id(generator_class, "getBlock", "(III)I")?;

        Ok(Self {
            generator_java_instance,
            get_block_method,
        })
    }

    pub fn get_block(&mut self, x: i32, y: i32, z: i32) -> i32 {
        let mut env = JVM.get_env().unwrap();
        unsafe {
            let x = jvalue { i: x };
            let y = jvalue { i: y };
            let z = jvalue { i: z };
            env.call_method_unchecked(
                &self.generator_java_instance,
                self.get_block_method,
                ReturnType::Primitive(Primitive::Int),
                &[x, y, z],
            )
            .unwrap()
            .i()
            .unwrap()
        }
    }
}
