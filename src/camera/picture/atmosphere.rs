use crate::simulation::coordinates::{GeographicCoordinates, HorizontalCoordinates};
use crate::utils::namespace::Namespace;
use crate::utils::numpy::{AnyArray, ArrayMethods, NewArray};
use pyo3::prelude::*;
use pyo3::types::{PyTuple, PyType};
use super::{RawPicture, Transform};
use super::colours::StandardRgb;
use super::DEFAULT_EXPOSURE;
use super::lights::{ResolvedLight, Light};
use super::pbr::{d_ggx, v_smith_ggx};
use super::vec3::Vec3;
use std::sync::OnceLock;


// ===============================================================================================
//
// Python interface to the atmosphere rendering.
//
// ===============================================================================================

const DEG: f64 = PI / 180.0;
const PI: f64 = std::f64::consts::PI;
const HALF_PI: f64 = 0.5 * PI;

/// Optical properties of the atmosphere.
#[pyclass(module="mulder.picture")]
pub struct SkyProperties;

#[pymethods]
impl SkyProperties {
    #[pyo3(signature=(picture, *lights))]
    #[classmethod]
    fn aerial_view<'py>(
        cls: &Bound<'py, PyType>,
        picture: &RawPicture,
        lights: &Bound<'py, PyTuple>,
    ) -> PyResult<Namespace<'py>> {
        let py = cls.py();
        let position = picture.position();
        let lights = {
            let mut directionals = Vec::new();
            for light in lights {
                let light: Light = light.extract()?;
                match light {
                    Light::Directional(light) => {
                        directionals.push(light.resolve(position))
                    },
                    Light::Sun(light) => {
                        directionals.push(
                            light.to_directional(position.latitude)?.resolve(position)
                        )
                    },
                    _ => (),
                }
            }
            directionals
        };

        let aerial = AerialView::new(&picture.transform, &lights);
        let mut u_array = NewArray::<f64>::empty(py, [AerialView::SHAPE.0])?;
        let mut v_array = NewArray::<f64>::empty(py, [AerialView::SHAPE.1])?;
        let mut distance_array = NewArray::<f64>::empty(py, [AerialView::SHAPE.2])?;
        let mut light_array = NewArray::<f64>::empty(
            py, [AerialView::SHAPE.2, AerialView::SHAPE.1, AerialView::SHAPE.0, 3])?;

        let u = u_array.as_slice_mut();
        for (i, ui) in aerial.0.iter_u().enumerate() {
            u[i] = ui;
        }
        let v = v_array.as_slice_mut();
        for (i, vi) in aerial.0.iter_v().enumerate() {
            v[i] = vi;
        }
        let distance = distance_array.as_slice_mut();
        for (i, wi) in aerial.0.iter_w().enumerate() {
            distance[i] = AerialView::unmap(wi);
        }
        let light = light_array.as_slice_mut();
        for (i, l) in aerial.0.data.iter().enumerate() {
            let hdr = *l * DEFAULT_EXPOSURE;
            let srgb = StandardRgb::from(hdr);
            light[3 * i + 0] = srgb.red();
            light[3 * i + 1] = srgb.green();
            light[3 * i + 2] = srgb.blue();
        }

        Namespace::new(py, [
            ("u", u_array),
            ("v", v_array),
            ("distance", distance_array),
            ("light", light_array),
        ])
    }

    #[classmethod]
    #[pyo3(signature=(/, *lights, latitude=None, longitude=None, altitude=None))]
    fn ambient_light<'py>(
        cls: &Bound<'py, PyType>,
        lights: &Bound<'py, PyTuple>,
        latitude: Option<f64>,
        longitude: Option<f64>,
        altitude: Option<f64>,
    ) -> PyResult<Namespace<'py>> {
        let py = cls.py();
        let latitude = latitude.unwrap_or(-45.0);
        let longitude = longitude.unwrap_or(0.0);
        let altitude = altitude.unwrap_or(0.0);
        let position = GeographicCoordinates { latitude, longitude, altitude };
        let lights = {
            let mut directionals = Vec::new();
            for light in lights {
                let light: Light = light.extract()?;
                match light {
                    Light::Directional(light) => {
                        directionals.push(light.resolve(&position))
                    },
                    Light::Sun(light) => {
                        directionals.push(light.to_directional(latitude)?.resolve(&position))
                    },
                    _ => (),
                }
            }
            directionals
        };

        let sky_view = SkyView::new(altitude, &lights);
        let average = sky_view.average();
        let diffuse = AmbientDiffuse::new(&average);
        let specular = AmbientSpecular::new(&average);
        let mut elevation_array = NewArray::<f64>::empty(py, [SkyView::AVERAGE_SIZE])?;
        let mut roughness_array = NewArray::<f64>::empty(py, [AmbientSpecular::SHAPE.1])?;
        let mut diffuse_array = NewArray::<f64>::empty(py, [SkyView::AVERAGE_SIZE, 3])?;
        let mut specular_array = NewArray::<f64>::empty(
            py, [SkyView::AVERAGE_SIZE, AmbientSpecular::SHAPE.1, 3]
        )?;

        let elevation = elevation_array.as_slice_mut();
        let roughness = roughness_array.as_slice_mut();
        let d = diffuse_array.as_slice_mut();
        let s = specular_array.as_slice_mut();
        for (i, v) in specular.0.iter_v().enumerate() {
            roughness[i] = v;
        }
        for (i, mu) in average.iter_u().enumerate() {
            elevation[i] = mu.asin() / DEG;
            let hdr = diffuse.eval(mu) * DEFAULT_EXPOSURE;
            let srgb = StandardRgb::from(hdr);
            d[3 * i + 0] = srgb.red();
            d[3 * i + 1] = srgb.green();
            d[3 * i + 2] = srgb.blue();

            for (j, roughness) in specular.0.iter_v().enumerate() {
                let hdr = specular.eval(mu, roughness.powi(2)) * DEFAULT_EXPOSURE;
                let srgb = StandardRgb::from(hdr);
                s[3 * (i * AmbientSpecular::SHAPE.1 + j) + 0] = srgb.red();
                s[3 * (i * AmbientSpecular::SHAPE.1 + j) + 1] = srgb.green();
                s[3 * (i * AmbientSpecular::SHAPE.1 + j) + 2] = srgb.blue();
            }
        }

        Namespace::new(py, [
            ("elevation", elevation_array),
            ("roughness", roughness_array),
            ("diffuse", diffuse_array),
            ("specular", specular_array),
        ])
    }

    #[classmethod]
    fn multiple_scattering<'py>(cls: &Bound<'py, PyType>) -> PyResult<Namespace<'py>> {
        let py = cls.py();

        let ms = MultipleScattering::get();
        let mut altitude_array = NewArray::<f64>::empty(py, [MultipleScattering::SHAPE.0])?;
        let mut elevation_array = NewArray::<f64>::empty(py, [MultipleScattering::SHAPE.1])?;
        let mut light_array = NewArray::<f64>::empty(
            py, [MultipleScattering::SHAPE.1, MultipleScattering::SHAPE.0, 3])?;

        let altitude = altitude_array.as_slice_mut();
        for (i, u) in ms.0.iter_u().enumerate() {
            altitude[i] = MultipleScattering::unmap_u(u) - Atmosphere::BOTTOM_RADIUS;
        }
        let elevation = elevation_array.as_slice_mut();
        for (i, v) in ms.0.iter_v().enumerate() {
            elevation[i] = MultipleScattering::unmap_v(v).asin() / DEG;
        }
        let light = light_array.as_slice_mut();
        for (i, l) in ms.0.data.iter().enumerate() {
            let hdr = *l * DEFAULT_EXPOSURE;
            let srgb = StandardRgb::from(hdr);
            light[3 * i + 0] = srgb.red();
            light[3 * i + 1] = srgb.green();
            light[3 * i + 2] = srgb.blue();
        }

        Namespace::new(py, [
            ("altitude", altitude_array),
            ("elevation", elevation_array),
            ("light", light_array),
        ])
    }

    #[classmethod]
    #[pyo3(signature=(/, *lights, latitude=None, longitude=None, altitude=None))]
    fn sky_view<'py>(
        cls: &Bound<'py, PyType>,
        lights: &Bound<'py, PyTuple>,
        latitude: Option<f64>,
        longitude: Option<f64>,
        altitude: Option<f64>,
    ) -> PyResult<Namespace<'py>> {
        let py = cls.py();
        let latitude = latitude.unwrap_or(-45.0);
        let longitude = longitude.unwrap_or(0.0);
        let altitude = altitude.unwrap_or(0.0);
        let position = GeographicCoordinates { latitude, longitude, altitude };
        let lights = {
            let mut directionals = Vec::new();
            for light in lights {
                let light: Light = light.extract()?;
                match light {
                    Light::Directional(light) => {
                        directionals.push(light.resolve(&position))
                    },
                    Light::Sun(light) => {
                        directionals.push(light.to_directional(latitude)?.resolve(&position))
                    },
                    _ => (),
                }
            }
            directionals
        };

        let sky_view = SkyView::new(altitude, &lights);
        let mut azimuth_array = NewArray::<f64>::empty(py, [SkyView::SHAPE.0])?;
        let mut elevation_array = NewArray::<f64>::empty(py, [SkyView::SHAPE.1])?;
        let mut light_array = NewArray::<f64>::empty(py, [SkyView::SHAPE.1, SkyView::SHAPE.0, 3])?;

        let azimuth = azimuth_array.as_slice_mut();
        for (i, u) in sky_view.0.iter_u().enumerate() {
            azimuth[i] = SkyView::unmap_u(u) / DEG;
        }
        let elevation = elevation_array.as_slice_mut();
        for (i, v) in sky_view.0.iter_v().enumerate() {
            elevation[i] = SkyView::unmap_v(v).asin() / DEG;
        }
        let light = light_array.as_slice_mut();
        for (i, l) in sky_view.0.data.iter().enumerate() {
            let hdr = *l * DEFAULT_EXPOSURE;
            let srgb = StandardRgb::from(hdr);
            light[3 * i + 0] = srgb.red();
            light[3 * i + 1] = srgb.green();
            light[3 * i + 2] = srgb.blue();
        }

        Namespace::new(py, [
            ("azimuth", azimuth_array),
            ("elevation", elevation_array),
            ("light", light_array),
        ])
    }

    #[pyo3(signature=(elevation, /, *, altitude=None))]
    #[classmethod]
    fn transmittance<'py>(
        _cls: &Bound<'py, PyType>,
        elevation: AnyArray<'py, f64>,
        altitude: Option<f64>,
    ) -> PyResult<NewArray<'py, f64>> {
        let py = elevation.py();

        let altitude = altitude.unwrap_or(0.0);
        let r = altitude + Atmosphere::BOTTOM_RADIUS;

        let mut shape = elevation.shape();
        shape.push(3);
        let mut array = NewArray::empty(py, shape)?;
        let values = array.as_slice_mut();
        let transmittance = Transmittance::get();
        for i in 0..elevation.size() {
            let mu = (elevation.get_item(i)? * DEG).sin();
            let ti = transmittance.eval(r, mu);
            for j in 0..3 {
                values[3 * i + j] = ti.0[j];
            }
        }

        Ok(array)
    }
}


// ===============================================================================================
//
// PBR implementation of the atmosphere
// Ref: Hillaire (2020), https://doi.org/10.1111/cgf.14050.
//
// ===============================================================================================

pub struct Atmosphere {
    aerial: AerialView,
    altitude: f64,
    ambient_diffuse: AmbientDiffuse,
    ambient_specular: AmbientSpecular,
    lights: Vec<ResolvedLight>,
    sky: SkyView,
    transmittance: &'static Transmittance,
}

struct AerialView (Lut3);

struct AmbientDiffuse (Vec3);

struct AmbientSpecular (Lut2);

struct MultipleScattering (Lut2);

struct SkyView (Lut2);

struct Transmittance (Lut2);

struct CrossSection {
    rayleigh_scattering: Vec3,
    mie_scattering: f64,
    extinction: Vec3,
}

struct Lut1 {
    size: usize,
    data: Vec<Vec3>,
}

struct Lut2 {
    shape: (usize, usize),
    data: Vec<Vec3>,
}

struct Lut3 {
    shape: (usize, usize, usize),
    data: Vec<Vec3>,
}

struct Iter1 {
    dx: f64,
    i: usize,
    n: usize,
}

struct IterMut1<'a> {
    du: f64,
    i: usize,
    iter: std::slice::IterMut<'a, Vec3>,
}

struct IterMut2<'a> {
    du: f64,
    dv: f64,
    iu: usize,
    iv: usize,
    nu: usize,
    iter: std::slice::IterMut<'a, Vec3>,
}

impl Atmosphere {
    #[inline]
    pub fn aerial_view(&self, u: f64, v: f64, distance: f64) -> Vec3 {
        self.aerial.eval(u, v, distance)
    }

    #[inline]
    pub fn ambient_diffuse(&self, elevation: f64) -> Vec3 {
        let mu = (elevation * DEG).sin();
        if mu < 0.0 {
            Vec3::ZERO
        } else {
            self.ambient_diffuse.eval(mu)
        }
    }

    #[inline]
    pub fn ambient_specular(&self, elevation: f64, roughness: f64) -> Vec3 {
        let mu = (elevation * DEG).sin();
        if mu < 0.0 {
            Vec3::ZERO
        } else {
            self.ambient_specular.eval(mu, roughness)
        }
    }

    pub fn new(picture: &RawPicture, lights: &[ResolvedLight]) -> Self {
        let aerial = AerialView::new(&picture.transform, lights);
        let altitude = picture.transform.frame.origin.altitude;
        let sky = SkyView::new(altitude, lights);
        let transmittance = Transmittance::get();
        let average = sky.average();
        let ambient_diffuse = AmbientDiffuse::new(&average);
        let ambient_specular = AmbientSpecular::new(&average);
        let lights = lights.into_iter().cloned().collect();
        Self { aerial, altitude, ambient_diffuse, ambient_specular, lights, sky, transmittance }
    }

    pub fn sky_view(&self, direction: &HorizontalCoordinates) -> Vec3 {
        let azimuth = {
            let mut azimuth = direction.azimuth * DEG;
            while azimuth < -PI {
                azimuth += 2.0 * PI;
            }
            while azimuth > PI {
                azimuth -= 2.0 * PI;
            }
            azimuth
        };
        let mu = (direction.elevation * DEG).sin();
        self.sky.eval(mu, azimuth)
    }

    pub fn sun_view(&self, elevation: f64, view: &[f64; 3]) -> Vec3 {
        const HALF_WIDTH: f64 = 0.5 * 0.545 * DEG;
        const SOLID_ANGLE: f64 = HALF_WIDTH * HALF_WIDTH * PI;

        let mut luminance = Vec3::ZERO;
        for light in self.lights.iter() {
            let nv = -Vec3::dot(&Vec3(light.direction), &Vec3(*view));
            let angle = nv.acos();
            if angle <= HALF_WIDTH {
                // Limb factor.
                // Ref: Hilaire 2016 (Nec96 model).
                const A: [f64; 3] = [0.397, 0.503, 0.652];
                let mu = (1.0 - (angle / HALF_WIDTH).powi(2)).sqrt();
                let limb_factor = Vec3::new(
                    mu.powf(A[0]),
                    mu.powf(A[1]),
                    mu.powf(A[2]),
                );

                let transmittance = self.transmittance(self.altitude, elevation);
                luminance += light.illuminance * transmittance * limb_factor;
            }
        }
        luminance / SOLID_ANGLE
    }

    pub fn transmittance(&self, altitude: f64, elevation: f64) -> Vec3 {
        let r = Atmosphere::BOTTOM_RADIUS + altitude;
        let mu = (elevation * DEG).sin();
        self.transmittance.eval(r, mu)
    }
}

impl Atmosphere {
    const BOTTOM_RADIUS: f64 = 6.36E+06;
    const TOP_RADIUS: f64 = 6.46E+06;

    const RAYLEIGH_SCALE: f64 = 1.0 / 8E+03;
    const RAYLEIGH_SCATTERING: Vec3 = Vec3::new(5.802E-06, 13.558E-06, 33.100E-06);

    const MIE_SCALE: f64 = 1.0 / 1.2E+03;
    const MIE_SCATTERING: f64 = 3.996E-06;
    const MIE_ABSORPTION: f64 = 4.40E-06;
    const MIE_ASYMMETRY: f64 = 0.8;

    const OZONE_ALTITUDE: f64 = 25E+03;
    const OZONE_WIDTH: f64 = 30E+03;
    const OZONE_ABSORPTION: Vec3 = Vec3::new(0.650E-06, 1.881E-06, 0.085E-06);

    const GROUND_ALBEDO: Vec3 = Vec3::splat(0.3);

    fn cross_section(r: f64) -> CrossSection {
        let altitude = r.clamp(Self::BOTTOM_RADIUS, Self::TOP_RADIUS) - Self::BOTTOM_RADIUS;

        let mie_density = (-Self::MIE_SCALE * altitude).exp();
        let rayleigh_density = (-Self::RAYLEIGH_SCALE * altitude).exp();
        let ozone_density = (1.0 -
            (altitude - Self::OZONE_ALTITUDE).abs() / (0.5 * Self::OZONE_WIDTH)).max(0.0);

        let mie_scattering = mie_density * Self::MIE_SCATTERING;
        let mie_absorption = mie_density * Self::MIE_ABSORPTION;
        let mie_extinction = mie_scattering + mie_absorption;
        let rayleigh_scattering = rayleigh_density * Self::RAYLEIGH_SCATTERING;
        let ozone_absorption = ozone_density * Self::OZONE_ABSORPTION;
        let extinction = rayleigh_scattering + mie_extinction + ozone_absorption;

        CrossSection { rayleigh_scattering, mie_scattering, extinction }
    }

    fn direction(mu: f64, azimuth: f64) -> Vec3 {
        let sin_theta = (1.0 - mu.powi(2)).sqrt();
        let phi = 0.5 * PI - azimuth;
        Vec3::new(sin_theta * phi.cos(), sin_theta * phi.sin(), mu)
    }

    fn distance_to_top(r: f64, mu: f64) -> f64 {
        let d = (r * r * (mu * mu - 1.0) + Self::TOP_RADIUS * Self::TOP_RADIUS)
            .max(0.0)
            .sqrt();
        (d - r * mu).max(0.0)
    }

    fn distance_to_bottom(r: f64, mu: f64) -> f64 {
        let d = (r * r * (mu * mu - 1.0) + Self::BOTTOM_RADIUS * Self::BOTTOM_RADIUS)
            .max(0.0)
            .sqrt();
        (-r * mu - d).max(0.0)
    }

    fn intersects_ground(r: f64, mu: f64) -> bool {
        (mu < 0.0) && (r * r * (mu * mu - 1.0) + Self::BOTTOM_RADIUS * Self::BOTTOM_RADIUS >= 0.0)
    }

    fn local_up(r: f64, t: f64, ray_dir: Vec3) -> Vec3 {
        (Vec3::new(0.0, 0.0, r) + t * ray_dir).normalize()
    }

    fn local_r(r: f64, mu: f64, t: f64) -> f64 {
        (t * t + 2.0 * r * mu * t + r * r).sqrt()
    }

    fn local_inscattering(
        cross_section: &CrossSection,
        ray_dir: &Vec3,
        local_r: f64,
        lights: &[ResolvedLight],
        multiple_scattering: Option<&MultipleScattering>,
        transmittance: &Transmittance,
    ) -> Vec3 {
        let mut inscattering = Vec3::ZERO;
        for light in lights.iter() {
            let mu_light = (light.elevation * DEG).sin();

            // Single scattering.
            let single_scattering = if Atmosphere::intersects_ground(local_r, mu_light) {
                Vec3::ZERO
            } else {
                let l = Atmosphere::direction(mu_light, light.azimuth * DEG);
                let lv = Vec3::dot(&l, ray_dir);
                let rayleigh_phase = Atmosphere::rayleigh(lv);
                let mie_phase = Atmosphere::henyey_greenstein(lv);
                let scattering_coeff = cross_section.rayleigh_scattering * rayleigh_phase +
                    cross_section.mie_scattering * mie_phase;
                let transmittance_to_light = transmittance.eval(local_r, mu_light);
                scattering_coeff * transmittance_to_light
            };

            // Multiple scattering.
            let multiple_scattering = match multiple_scattering {
                Some(multiple_scattering) => {
                    let psi_ms = multiple_scattering.eval(local_r, mu_light);
                    psi_ms * (cross_section.rayleigh_scattering + cross_section.mie_scattering)
                },
                None => Vec3::ZERO,
            };

            inscattering += light.illuminance * (single_scattering + multiple_scattering);
        }
        inscattering
    }

    fn max_distance(r: f64, mu: f64) -> f64 {
        if Self::intersects_ground(r, mu) {
            Self::distance_to_bottom(r, mu)
        } else {
            Self::distance_to_top(r, mu)
        }
    }

    fn rayleigh(lv: f64) -> f64 {
        3.0 / (16.0 * PI) * (1.0 + lv.powi(2))
    }

    fn henyey_greenstein(lv: f64) -> f64 {
        let g = Self::MIE_ASYMMETRY;
        let denom = 1.0 + g * g - 2.0 * g * lv;
        (1.0 - g * g) / (4.0 * PI * denom * denom.sqrt())
    }
}

impl AerialView {
    const MAX_DISTANCE: f64 = 1E+05;
    const MIN_DISTANCE: f64 = 1E+01;
    const SHAPE: (usize, usize, usize) = (32, 32, 32);

    pub fn eval(&self, u: f64, v: f64, distance: f64) -> Vec3 {
        let w = Self::map(distance);
        self.0.interpolate(u, v, w)
    }

    fn map(distance: f64) -> f64 {
        if distance <= Self::MIN_DISTANCE {
            0.0
        } else if distance >= Self::MAX_DISTANCE {
            1.0
        } else {
            (distance / Self::MIN_DISTANCE).ln() / (Self::MAX_DISTANCE / Self::MIN_DISTANCE).ln()
        }
    }

    fn new(transform: &Transform, lights: &[ResolvedLight]) -> Self {
        let r = transform.frame.origin.altitude + Atmosphere::BOTTOM_RADIUS;
        let mut lut = Lut3::new(Self::SHAPE);
        let transmittance = Transmittance::get();
        for (iv, v) in lut.iter_v().enumerate() {
            for (iu, u) in lut.iter_u().enumerate() {
                let (mu, azimuth) = {
                    let direction = transform.direction(u, v);
                    let mu = (direction.elevation * DEG).sin();
                    let azimuth = direction.azimuth * DEG;
                    (mu, azimuth)
                };
                let ray_dir = Atmosphere::direction(mu, azimuth);
                let t_max = Atmosphere::max_distance(r, mu);

                let mut total_inscattering = Vec3::ZERO;
                let mut throughput = Vec3::splat(1.0);
                let mut prev_t = 0.0;
                let mut saturate = false;
                for (iw, w) in lut.iter_w().enumerate() {
                    if saturate {
                        lut.set(iu, iv, iw, total_inscattering);
                        continue;
                    }

                    let t_i = Self::unmap(w).min(t_max);
                    let dt_i = t_i - prev_t;
                    prev_t = t_i;

                    let local_r = Atmosphere::local_r(r, mu, t_i);
                    let cross_section = Atmosphere::cross_section(local_r);

                    let sample_optical_depth = cross_section.extinction * dt_i;
                    let sample_transmittance = (-sample_optical_depth).exp();

                    let inscattering = Atmosphere::local_inscattering(
                        &cross_section,
                        &ray_dir,
                        local_r,
                        lights,
                        None,
                        transmittance,
                    );

                    // Analytical integration of the single scattering term in the radiance
                    // transfer equation.
                    let s_int = (inscattering - inscattering * sample_transmittance) /
                        cross_section.extinction;
                    total_inscattering += throughput * s_int;
                    lut.set(iu, iv, iw, total_inscattering);

                    throughput *= sample_transmittance;
                    const THRESHOLD: f64 = 0.001;
                    if (throughput.x() < THRESHOLD) &&
                       (throughput.y() < THRESHOLD) &&
                       (throughput.z() < THRESHOLD) {
                        saturate = true;
                    }
                }
            }
        }
        Self (lut)
    }

    fn unmap(w: f64) -> f64 {
        Self::MIN_DISTANCE * (w * (Self::MAX_DISTANCE / Self::MIN_DISTANCE).ln()).exp()
    }
}

impl AmbientDiffuse {
    pub fn eval(&self, mu: f64) -> Vec3 {
        let mu = mu.clamp(0.0, 1.0);
        mu * self.0
    }

    fn new(sky: &Lut1) -> Self {
        let mut cz = Vec3::ZERO;
        for (i, mu) in sky.iter_u().enumerate() {
            let si = sky.data[i];
            cz += mu * si;
        }
        cz  *= 2.0 / (sky.size as f64);
        Self (cz)
    }
}

impl AmbientSpecular {
    const SHAPE: (usize, usize) = (32, 32);
    const N_PHI: usize = 16;
    const N_THETA: usize = 32;

    fn eval(&self, mu: f64, alpha: f64) -> Vec3 {
        self.0.interpolate(mu, alpha)
    }

    fn new(sky: &Lut1) -> Self {
        let mut lut = Lut2::new(Self::SHAPE);
        for (c_n, roughness, d) in lut.iter_mut() {
            let mut num = Vec3::ZERO;
            let mut den = 0.0;
            let a2 = roughness * roughness;
            let s_n = (1.0 - c_n.powi(2)).sqrt();
            for i in 0..Self::N_THETA {
                let c = i as f64 / (Self::N_THETA - 1) as f64;
                let s = (1.0 - c.powi(2)).sqrt();
                let dvnl = d_ggx(0.5 * (1.0 + c), a2) * v_smith_ggx(1.0, c, a2) * c;
                let mut sky_i = Vec3::ZERO;
                for j in 0..Self::N_PHI {
                    let phi = (j as f64) * 2.0 * PI / (Self::N_PHI - 1) as f64;
                    let c_l = c * c_n - s * s_n * phi.cos();
                    if c_l > 0.0 {
                        sky_i += sky.interpolate(c_l);
                    }
                }
                num += sky_i * (dvnl / Self::N_PHI as f64);
                den += dvnl;
            }
            *d = if den > 0.0 { num / den } else { Vec3::ZERO };
        }
        Self (lut)
    }
}


static MULTIPLE_SCATTERING: OnceLock<MultipleScattering> = OnceLock::new();

impl MultipleScattering {
    const DIRECTIONS: usize = 64;
    const SHAPE: (usize, usize) = (32, 32);
    const SAMPLES: usize = 16;

    fn eval(&self, r: f64, mu: f64) -> Vec3 {
        let (u, v) = Self::map(r, mu);
        self.0.interpolate(u, v)
    }

    fn get() -> &'static Self {
        MULTIPLE_SCATTERING.get_or_init(|| Self::new())
    }

    fn map(r: f64, mu: f64) -> (f64, f64) {
        let u = ((r - Atmosphere::BOTTOM_RADIUS) /
            (Atmosphere::TOP_RADIUS - Atmosphere::BOTTOM_RADIUS))
            .clamp(0.0, 1.0);
        let v = 0.5 + 0.5 * mu;
        (u, v)
    }

    fn new() -> Self {
        let mut lut = Lut2::new(Self::SHAPE);
        let transmittance = Transmittance::get();
        for (u, v, d) in lut.iter_mut() {
            let (r, mu) = Self::unmap(u, v);
            let light_dir = Atmosphere::direction(mu, 0.0);
            let mut l_2 = Vec3::ZERO;
            let mut f_ms = Vec3::ZERO;
            for i in 0..Self::DIRECTIONS {
                const PHI_X: f64 = 1.3247179572447460259609088;
                const PHI_Y: f64 = 1.7548776662466927600495087;
                let x = (0.5 + (i as f64) * PHI_X).fract();
                let y = (0.5 + (i as f64) * PHI_Y).fract();
                let ray_dir = Atmosphere::direction(2.0 * x - 1.0, 2.0 * PI * y);
                let sample = Self::sample_dir(r, ray_dir, light_dir, transmittance);
                l_2 += sample.0;
                f_ms += sample.1;
            }
            l_2 /= Self::DIRECTIONS as f64;
            f_ms /= Self::DIRECTIONS as f64;
            *d = l_2 / (1.0 - f_ms);
        }
        Self (lut)
    }

    fn sample_dir(
        r: f64,
        ray_dir: Vec3,
        light_dir: Vec3,
        transmittance: &Transmittance,
    ) -> (Vec3, Vec3) {
        let mu_view = ray_dir.z();
        let t_max = Atmosphere::max_distance(r, mu_view);
        let dt = t_max / (Self::SAMPLES as f64);
        let mut optical_depth = Vec3::ZERO;
        let mut l_2 = Vec3::ZERO;
        let mut f_ms = Vec3::ZERO;
        let mut  throughput = Vec3::splat(1.0);
        for i in 0..Self::SAMPLES {
            let t_i = dt * (i as f64 + 0.5);
            let local_r = Atmosphere::local_r(r, mu_view, t_i);
            let local_up = Atmosphere::local_up(r, t_i, ray_dir);

            let cross_section = Atmosphere::cross_section(local_r);
            let sample_optical_depth = cross_section.extinction * dt;
            let sample_transmittance = (-sample_optical_depth).exp();
            optical_depth += sample_optical_depth;

            let mu_light = Vec3::dot(&light_dir, &local_up);
            if !Atmosphere::intersects_ground(local_r, mu_light) {
                let scattering_no_phase =
                    cross_section.rayleigh_scattering + cross_section.mie_scattering;

                let ms_int = (scattering_no_phase - scattering_no_phase * sample_transmittance) /
                    cross_section.extinction;
                f_ms += throughput * ms_int;

                let transmittance_to_light = transmittance.eval(local_r, mu_light);

                let s = scattering_no_phase * transmittance_to_light / (4.0 * PI);
                let s_int = (s - s * sample_transmittance) / cross_section.extinction;
                l_2 += throughput * s_int;
            }

            throughput *= sample_transmittance;
            const THRESHOLD: f64 = 0.001;
            if (throughput.x() < THRESHOLD) &&
               (throughput.y() < THRESHOLD) &&
               (throughput.z() < THRESHOLD) {
                break
            }
        }

        if Atmosphere::intersects_ground(r, mu_view) {
            let transmittance_to_ground = (-optical_depth).exp();
            let local_up = Atmosphere::local_up(r, t_max, ray_dir);
            let mu_light = Vec3::dot(&light_dir, &local_up);
            let transmittance_to_light = transmittance.eval(0.0, mu_light);
            let ground_luminance = transmittance_to_light * transmittance_to_ground *
                mu_light.max(0.0) * Atmosphere::GROUND_ALBEDO;
            l_2 += ground_luminance;
        }

        (l_2, f_ms)
    }

    fn unmap(u: f64, v: f64) -> (f64, f64) {
        let r = Self::unmap_u(u);
        let mu = Self::unmap_v(v);
        (r, mu)
    }

    fn unmap_u(u: f64) -> f64 {
        (Atmosphere::TOP_RADIUS - Atmosphere::BOTTOM_RADIUS) * u + Atmosphere::BOTTOM_RADIUS
    }

    fn unmap_v(v: f64) -> f64 {
        2.0 * v - 1.0
    }
}

impl SkyView {
    const AVERAGE_SIZE: usize = 91;
    const SHAPE: (usize, usize) = (360, 180);
    const SAMPLES: usize = 16;
    const MIDPOINT_RATIO: f64 = 0.3;

    fn average(&self) -> Lut1 {
        let mut lut = Lut1::new(Self::AVERAGE_SIZE);
        for (mu, d) in lut.iter_mut() {
            let mut di = Vec3::ZERO;
            for u in self.0.iter_u() {
                let azimuth = Self::unmap_u(u);
                di += self.eval(mu, azimuth);
            }
            *d = di / (Self::SHAPE.0 as f64);
        }
        lut
    }

    fn eval(&self, mu: f64, azimuth: f64) -> Vec3 {
        let (u, v) = Self::map(mu, azimuth);
        self.0.interpolate(u, v)
    }

    fn map(mu: f64, azimuth: f64) -> (f64, f64) {
        let u = azimuth / (2.0 * PI) + 0.5;
        let l = mu.asin();
        let abs_l = l.abs();
        let v = 0.5 + 0.5 * l.signum() * (abs_l / HALF_PI).sqrt();
        (u, v)
    }

    fn new(altitude: f64, lights: &[ResolvedLight]) -> Self {
        let r = altitude + Atmosphere::BOTTOM_RADIUS;
        let mut lut = Lut2::new(Self::SHAPE);
        let multiple_scattering = MultipleScattering::get();
        let transmittance = Transmittance::get();
        for (u, v, d) in lut.iter_mut() {
            let (mu, azimuth) = Self::unmap(u, v);
            let ray_dir = Atmosphere::direction(mu, azimuth);
            let t_max = Atmosphere::max_distance(r, mu);
            let n = 1.0 + (Self::SAMPLES as f64 - 1.0) * (t_max * 0.01).clamp(0.0, 1.0);
            let n = n as usize;
            let mut total_inscattering = Vec3::ZERO;
            let mut throughput = Vec3::splat(1.0);
            let mut prev_t = 0.0;
            for i in 0..n {
                let t_i = t_max * (i as f64 + Self::MIDPOINT_RATIO) / (n as f64);
                let dt_i = t_i - prev_t;
                prev_t = t_i;

                let local_r = Atmosphere::local_r(r, mu, t_i);
                let cross_section = Atmosphere::cross_section(local_r);

                let sample_optical_depth = cross_section.extinction * dt_i;
                let sample_transmittance = (-sample_optical_depth).exp();

                let inscattering = Atmosphere::local_inscattering(
                    &cross_section,
                    &ray_dir,
                    local_r,
                    lights,
                    Some(multiple_scattering),
                    transmittance,
                );

                // Analytical integration of the single scattering term in the radiance transfer
                // equation.
                let s_int = (inscattering - inscattering * sample_transmittance) /
                    cross_section.extinction;
                total_inscattering += throughput * s_int;

                throughput *= sample_transmittance;
                const THRESHOLD: f64 = 0.001;
                if (throughput.x() < THRESHOLD) &&
                   (throughput.y() < THRESHOLD) &&
                   (throughput.z() < THRESHOLD) {
                    break
                }
            }
            *d = total_inscattering;
        }
        Self (lut)
    }

    fn unmap(u: f64, v: f64) -> (f64, f64) {
        let azimuth = Self::unmap_u(u);
        let mu = Self::unmap_v(v);
        (mu, azimuth)
    }

    fn unmap_u(u: f64) -> f64 {
        (2.0 * u - 1.0) * PI
    }

    fn unmap_v(v: f64) -> f64 {
        let t = (2.0 * (v - 0.5)).abs();
        let l = (v - 0.5).signum() * HALF_PI * t * t;
        l.sin()
    }
}

static TRANSMITTANCE: OnceLock<Transmittance> = OnceLock::new();

impl Transmittance {
    const SHAPE: (usize, usize) = (256, 128);
    const SAMPLES: usize = 40;
    const MIDPOINT_RATIO: f64 = 0.3;

    fn eval(&self, r: f64, mu: f64) -> Vec3 {
        let (u, v) = Self::map(r, mu);
        self.0.interpolate(u, v)
    }

    fn get() -> &'static Self {
        TRANSMITTANCE.get_or_init(|| Self::new())
    }

    fn map(r: f64, mu: f64) -> (f64, f64) {
        const R02: f64 = Atmosphere::BOTTOM_RADIUS * Atmosphere::BOTTOM_RADIUS;
        const R12: f64 = Atmosphere::TOP_RADIUS * Atmosphere::TOP_RADIUS;
        let h = (R12 - R02).sqrt();
        let rho = (r * r - R02).max(0.0).sqrt();
        let d = Atmosphere::distance_to_top(r, mu);
        let d_min = Atmosphere::TOP_RADIUS - r;
        let d_max = rho + h;
        let u = (d - d_min) / (d_max - d_min);
        let v = rho / h;
        (u, v)
    }

    fn new() -> Self {
        let mut data = Lut2::new(Self::SHAPE);
        for (u, v, d) in data.iter_mut() {
            let (r, mu) = Self::unmap(u, v);

            let t_max = Atmosphere::distance_to_top(r, mu);  // the ground should be ignored.
            let mut optical_depth = Vec3::ZERO;
            let mut prev_t = 0.0;
            for i in 0..Self::SAMPLES {
                let t_i = t_max * ((i as f64 + Self::MIDPOINT_RATIO) / (Self::SAMPLES as f64));
                let dt = t_i - prev_t;
                prev_t = t_i;
                let r_i = Atmosphere::local_r(r, mu, t_i);
                let cross_section = Atmosphere::cross_section(r_i);
                optical_depth += cross_section.extinction * dt;
            }
            *d = (-optical_depth).exp();
        }
        Self (data)
    }

    fn unmap(u: f64, v: f64) -> (f64, f64) {
        const R02: f64 = Atmosphere::BOTTOM_RADIUS * Atmosphere::BOTTOM_RADIUS;
        const R12: f64 = Atmosphere::TOP_RADIUS * Atmosphere::TOP_RADIUS;
        let h = (R12 - R02).sqrt();
        let rho = h * v;
        let r = (rho * rho + R02).sqrt();
        let d_min = Atmosphere::TOP_RADIUS - r;
        let d_max = rho + h;
        let d = d_min + u * (d_max - d_min);

        let mu = if d == 0.0 {
            1.0
        } else {
            ((h * h - rho * rho - d * d) / (2.0 * r * d))
                .clamp(-1.0, 1.0)
        };
        (r, mu)
    }
}

impl Lut1 {
    fn interpolate(&self, u: f64) -> Vec3 {
        let n = self.size;
        let (i, h) = compute_index(u, n);
        let f0 = self.data[i];
        let f1 = self.data[i + 1];
        f0 * (1.0 - h) + f1 * h
    }

    fn iter_mut<'a>(&'a mut self) -> IterMut1<'a> {
        let n = self.size;
        let du = 1.0 / (n - 1) as f64;
        let i = 0;
        let iter = self.data.iter_mut();
        IterMut1 { du, i, iter }
    }

    fn iter_u(&self) -> Iter1 {
        Iter1::new(self.size)
    }

    fn new(size: usize) -> Self {
        let data = vec![Vec3::ZERO; size];
        Self { size, data }
    }
}

impl Lut2 {
    fn interpolate(&self, u: f64, v: f64) -> Vec3 {
        let (nu, nv) = self.shape;
        let (iu, hu) = compute_index(u, nu);
        let (iv, hv) = compute_index(v, nv);
        let f00 = self.data[iv * nu + iu];
        let f01 = self.data[(iv + 1) * nu + iu];
        let f10 = self.data[iv * nu + iu + 1];
        let f11 = self.data[(iv + 1) * nu + iu + 1];
        (f00 * (1.0 - hu) + f10 * hu) * (1.0 - hv) +
        (f01 * (1.0 - hu) + f11 * hu) * hv
    }

    fn iter_mut<'a>(&'a mut self) -> IterMut2<'a> {
        let (nu, nv) = self.shape;
        let du = 1.0 / (nu - 1) as f64;
        let dv = 1.0 / (nv - 1) as f64;
        let iu = 0;
        let iv = 0;
        let iter = self.data.iter_mut();
        IterMut2 { du, dv, iu, iv, nu, iter }
    }

    fn iter_u(&self) -> Iter1 {
        Iter1::new(self.shape.0)
    }

    fn iter_v(&self) -> Iter1 {
        Iter1::new(self.shape.1)
    }

    fn new(shape: (usize, usize)) -> Self {
        let data = vec![Vec3::ZERO; shape.0 * shape.1];
        Self { shape, data }
    }
}

impl Lut3 {
    fn interpolate(&self, u: f64, v: f64, w: f64) -> Vec3 {
        let (nu, nv, nw) = self.shape;
        let (iu, hu) = compute_index(u, nu);
        let (iv, hv) = compute_index(v, nv);
        let (iw, hw) = compute_index(w, nw);
        let f000 = self.data[(iw * nv + iv) * nu + iu];
        let f010 = self.data[(iw * nv + iv + 1) * nu + iu];
        let f100 = self.data[(iw * nv + iv) * nu + iu + 1];
        let f110 = self.data[(iw * nv + iv + 1) * nu + iu + 1];
        let f001 = self.data[((iw + 1) * nv + iv) * nu + iu];
        let f011 = self.data[((iw + 1) * nv + iv + 1) * nu + iu];
        let f101 = self.data[((iw + 1) * nv + iv) * nu + iu + 1];
        let f111 = self.data[((iw + 1) * nv + iv + 1) * nu + iu + 1];
        (
            (f000 * (1.0 - hu) + f100 * hu) * (1.0 - hv) +
            (f010 * (1.0 - hu) + f110 * hu) * hv
        ) * (1.0 - hw) + (
            (f001 * (1.0 - hu) + f101 * hu) * (1.0 - hv) +
            (f011 * (1.0 - hu) + f111 * hu) * hv
        ) * hw
    }

    fn iter_u(&self) -> Iter1 {
        Iter1::new(self.shape.0)
    }

    fn iter_v(&self) -> Iter1 {
        Iter1::new(self.shape.1)
    }

    fn iter_w(&self) -> Iter1 {
        Iter1::new(self.shape.2)
    }

    fn new(shape: (usize, usize, usize)) -> Self {
        let data = vec![Vec3::ZERO; shape.0 * shape.1 * shape.2];
        Self { shape, data }
    }

    fn set(&mut self, iu: usize, iv: usize, iw: usize, value: Vec3) {
        let (nu, nv, _) = self.shape;
        self.data[(iw * nv + iv) * nu + iu] = value;
    }
}

impl Iter1 {
    const fn new(n: usize) -> Self {
        let dx = 1.0 / (n - 1) as f64;
        let i = 0;
        Self { dx, i, n }
    }
}

impl Iterator for Iter1 {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i < self.n {
            let x = (self.i as f64) * self.dx;
            self.i += 1;
            Some(x)
        } else {
            None
        }
    }
}

impl<'a> Iterator for IterMut1<'a> {
    type Item = (f64, &'a mut Vec3);

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            Some(d) => {
                let u = (self.i as f64) * self.du;
                self.i += 1;
                Some((u, d))
            },
            None => None,
        }
    }
}

impl<'a> Iterator for IterMut2<'a> {
    type Item = (f64, f64, &'a mut Vec3);

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            Some(d) => {
                let u = (self.iu as f64) * self.du;
                let v = (self.iv as f64) * self.dv;
                let iu = self.iu + 1;
                if iu >= self.nu {
                    self.iv += 1;
                    self.iu = 0;
                } else {
                    self.iu = iu;
                }
                Some((u, v, d))
            },
            None => None,
        }
    }
}

#[inline]
fn compute_index(x: f64, n: usize) -> (usize, f64) {
    let mut h = x.clamp(0.0, 1.0) * (n - 1) as f64;
    let i = h as usize;
    if i >= n - 1 {
        (n - 2, 1.0)
    } else {
        h -= i as f64;
        (i, h)
    }
}
