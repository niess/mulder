#![allow(non_snake_case)]

use super::atmosphere::Atmosphere;
use super::materials::MaterialData;
use super::lights::ResolvedLight;
use super::vec3::Vec3;


const PI: f64 = std::f64::consts::PI;

pub fn illuminate(
    u: f64,
    v: f64,
    altitude: f64,
    distance: f64,
    normal: [f64;3],
    view: [f64;3],
    ambient_light: Vec3,
    directional_lights: &[ResolvedLight],
    material: &MaterialData,
    atmosphere: Option<&Atmosphere>,
) -> Vec3 {
    let diffuse_colour = Vec3(material.diffuse_colour);
    let f0 = Vec3(material.f0);
    let normal = Vec3(normal);
    let view = Vec3(view);
    let nv = Vec3::dot(&normal, &view).max(1E-04);
    let dfg = dfg_approx(nv, material.perceptual_roughness);
    let r = dfg.0 + dfg.1;

    let mut directional = Vec3::ZERO;
    for light in directional_lights {
        let l = Vec3(light.direction);
        let nl = Vec3::dot(&normal, &l).clamp(0.0, 1.0);
        if nl <= 0.0 { continue }
        let brdf = brdf(
            l,
            normal,
            view,
            nl,
            nv,
            diffuse_colour,
            f0,
            material.roughness,
            r,
        );
        let mut li = brdf * light.illuminance * nl;
        if let Some(atmosphere) = atmosphere {
            li *= atmosphere.transmittance(altitude, light.elevation);
        }
        directional += li;
    }

    let ambient = if nv > 0.0 {
        let specular_ambient = dfg.0 * f0 + dfg.1;
        (diffuse_colour + specular_ambient) * ambient_light
    } else {
        Vec3::ZERO
    };

    let aerial = match atmosphere {
        Some(atmosphere) => atmosphere.aerial_view(u, v, distance),
        None => Vec3::ZERO,
    };

    (directional + aerial) * PI + ambient
}


// ===============================================================================================
//
// Implementation of Physically Based Rendering (PBR).
// Ref: https://google.github.io/filament/Filament.md.html#materialsystem/standardmodelsummary
//
// ===============================================================================================

fn brdf(
    l: Vec3,
    n: Vec3,
    v: Vec3,
    nl: f64,
    nv: f64,
    diffuse_color: Vec3,
    f0: Vec3,
    roughness: f64,
    r: f64,
) -> Vec3 {
    let h = Vec3::normalize(v + l);

    let nh = Vec3::dot(&n, &h).clamp(0.0, 1.0);
    let lh = Vec3::dot(&l, &h).clamp(0.0, 1.0);

    let a2 = roughness.powi(2);
    let d = d_ggx(nh, a2);
    let f = f_schlick(lh, f0);
    let v = v_smith_ggx(nv, nl, a2);

    // Specular BRDF.
    let mut fr = (d * v) * f;
    fr *= 1.0 + f0 * (1.0 / r - 1.0);

    // Diffuse BRDF.
    let fd = diffuse_color * f_lambert();

    fr + fd
}

#[inline]
fn d_ggx(nh: f64, a2: f64) -> f64 {
    let f = (nh * a2 - nh) * nh + 1.0;
    a2 / (PI * f * f)
}

#[inline]
fn f_schlick(u: f64, f0: Vec3) -> Vec3 {
    f0 + (Vec3::splat(1.0) - f0) * (1.0 - u).powf(5.0)
}

#[inline]
fn v_smith_ggx(nv: f64, nl: f64, a2: f64) -> f64 {
    let ggxv = nl * (nv.powi(2) * (1.0 - a2) + a2).sqrt();
    let ggxl = nv * (nl.powi(2) * (1.0 - a2) + a2).sqrt();
    0.5 / (ggxl + ggxv)
}

const fn f_lambert() -> f64 {
    1.0 / PI
}

// Ref: https://www.unrealengine.com/en-US/blog/physically-based-shading-on-mobile
fn dfg_approx(nv: f64, perceptual_roughness: f64) -> (f64, f64) {
    const C0: [f64; 4] = [-1.0, -0.0275, -0.572, 0.022];
    const C1: [f64; 4] = [1.0, 0.0425, 1.04, -0.04];
    let r = [
        perceptual_roughness * C0[0] + C1[0],
        perceptual_roughness * C0[1] + C1[1],
        perceptual_roughness * C0[2] + C1[2],
        perceptual_roughness * C0[3] + C1[3],
    ];
    let a004 = r[0].powi(2).min(2.0_f64.powf(-9.28 * nv)) * r[0] + r[1];
    let x = -1.04 * a004 + r[2];
    let y = 1.04 * a004 + r[3];
    (x, y)
}
