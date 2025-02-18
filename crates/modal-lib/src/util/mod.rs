#![allow(dead_code)]

use image::ImageFormat;

pub mod serde_rwlock {
    use serde::de::Deserializer;
    use serde::ser::Serializer;
    use serde::{Deserialize, Serialize};
    use std::sync::RwLock;

    pub fn serialize<S, T>(val: &RwLock<T>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        T::serialize(&*val.read().unwrap(), s)
    }

    pub fn deserialize<'de, D, T>(d: D) -> Result<RwLock<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        Ok(RwLock::new(T::deserialize(d)?))
    }
}

pub mod serde_mutex {
    use serde::de::Deserializer;
    use serde::ser::Serializer;
    use serde::{Deserialize, Serialize};
    use std::sync::Mutex;

    pub fn serialize<S, T>(val: &Mutex<T>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        T::serialize(&*val.lock().unwrap(), s)
    }

    pub fn deserialize<'de, D, T>(d: D) -> Result<Mutex<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        Ok(Mutex::new(T::deserialize(d)?))
    }
}

pub mod serde_smf {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    struct Smf(Vec<u8>);

    pub fn serialize<S>(val: &midly::Smf<'_>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut bytes = Vec::new();
        val.write(&mut bytes).unwrap();
        let smf_serializable = Smf(bytes);

        smf_serializable.serialize(s)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<midly::Smf<'static>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let smf_deserializable = Smf::deserialize(d)?;

        Ok(midly::Smf::parse(&smf_deserializable.0)
            .unwrap()
            .make_static())
    }
}

pub mod serde_pid {
    use num_traits::float::FloatCore;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    struct Pid {
        setpoint: f32,
        output_limit: f32,
        kp: f32,
        ki: f32,
        kd: f32,
        p_limit: f32,
        i_limit: f32,
        d_limit: f32,
    }

    impl Pid {
        fn as_pid(&self) -> pid::Pid<f32> {
            let mut pid = pid::Pid::new(self.setpoint, self.output_limit);
            pid.p(self.kp, self.p_limit)
                .i(self.ki, self.i_limit)
                .d(self.kd, self.d_limit);

            pid
        }
    }

    impl<'a, T: Into<f32> + FloatCore> From<&'a pid::Pid<T>> for Pid {
        fn from(pid: &'a pid::Pid<T>) -> Self {
            Pid {
                setpoint: pid.setpoint.into(),
                output_limit: pid.output_limit.into(),
                kp: pid.kp.into(),
                ki: pid.ki.into(),
                kd: pid.kd.into(),
                p_limit: pid.p_limit.into(),
                i_limit: pid.i_limit.into(),
                d_limit: pid.d_limit.into(),
            }
        }
    }

    pub fn serialize<T: FloatCore + Into<f32>, S>(
        val: &pid::Pid<T>,
        s: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Pid::from(val).serialize(s)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<pid::Pid<f32>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Pid::deserialize(d).map(|pid_deser| pid_deser.as_pid())
    }
}

pub mod serde_perlin {
    use noise::Seedable;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    struct Perlin(u32);

    pub fn serialize<S>(val: &noise::Perlin, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let perlin = Perlin(val.seed());

        perlin.serialize(s)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<noise::Perlin, D::Error>
    where
        D: Deserializer<'de>,
    {
        let perlin = Perlin::deserialize(d)?;

        Ok(noise::Perlin::new(perlin.0))
    }
}

#[macro_export]
macro_rules! serde_atomic_enum {
    ($ty:ident) => {
        impl ::serde::Serialize for $ty {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                self.0.serialize(serializer)
            }
        }

        impl<'de> serde::Deserialize<'de> for $ty {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                ::std::sync::atomic::AtomicUsize::deserialize(deserializer).map(|inner| $ty(inner))
            }
        }
    };
}

pub fn enum_combo_box<
    E: strum::IntoEnumIterator + std::fmt::Display + PartialEq + std::any::Any,
>(
    ui: &mut eframe::egui::Ui,
    e: &mut E,
) {
    eframe::egui::ComboBox::from_id_salt(e.type_id())
        .selected_text(format!("{e}"))
        .show_ui(ui, |ui| {
            for variant in E::iter() {
                let name = format!("{variant}");
                ui.selectable_value(e, variant, name);
            }
        });
}

pub mod perlin {
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct Perlin1D {
        rand_noise: Vec<f32>,
    }

    impl Default for Perlin1D {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Perlin1D {
        pub fn new() -> Self {
            Perlin1D {
                rand_noise: (0..44100).map(|_| rand::random()).collect(),
            }
        }

        fn fade(t: f32) -> f32 {
            t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
        }

        fn grad(&self, p: f32) -> f32 {
            let v = self.rand_noise[p.floor() as usize % self.rand_noise.len()];
            if v > 0.5 {
                1.0
            } else {
                -1.0
            }
        }

        pub fn noise(&self, p: f32) -> f32 {
            let p0 = p.floor();
            let p1 = p0 + 1.0;

            let t = p - p0;
            let fade_t = Self::fade(t);

            let g0 = self.grad(p0);
            let g1 = self.grad(p1);

            (1.0 - fade_t) * g0 * (p - p0) + fade_t * g1 * (p - p1)
        }
    }
}

pub fn toggle_button(label: &str, state: bool) -> eframe::egui::Button {
    if state {
        eframe::egui::Button::new(
            eframe::egui::RichText::new(label).color(eframe::epaint::Color32::BLACK),
        )
        .fill(eframe::epaint::Color32::GOLD)
    } else {
        eframe::egui::Button::new(label)
    }
}

pub fn load_image_from_path(bytes: &[u8]) -> eframe::egui::ColorImage {
    dbg!(bytes.len());
    let image = image::io::Reader::with_format(std::io::Cursor::new(bytes), ImageFormat::Png)
        .decode()
        .unwrap();
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    eframe::egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice())
}
