#![allow(non_snake_case)]

use super::atmosphere::Atmosphere;
use super::materials::MaterialData;
use super::lights::ResolvedLight;
use super::vec3::Vec3;


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
    let specular_colour = Vec3(material.f0);
    let normal = Vec3(normal);
    let view = Vec3(view);
    let NoV = max(Vec3::dot(&normal, &view), 1E-04);

    let mut luminance = Vec3::ZERO;
    for light in directional_lights {
        let l = Vec3(light.direction);
        let NoL = clamp(Vec3::dot(&normal, &l), 0.0, 1.0);
        let brdf = BRDF(
            l,
            normal,
            view,
            NoL,
            NoV,
            diffuse_colour,
            specular_colour,
            material.roughness,
        );

        let mut li = brdf * light.illuminance * NoL;
        if let Some(atmosphere) = atmosphere {
            li *= atmosphere.transmittance(altitude, light.elevation);
        }
        luminance += li;
    }
    if let Some(atmosphere) = atmosphere {
        luminance += atmosphere.aerial_view(u, v, distance);
    }

    let diffuse_ambient = EnvBRDFApprox(diffuse_colour, NoV, 1.0);
    let specular_ambient = EnvBRDFApprox(specular_colour, NoV, material.perceptual_roughness);
    let ambient = (diffuse_ambient + specular_ambient) * ambient_light;

    luminance + ambient
}


// ===============================================================================================
//
// Implementation of Physically Based Rendering (PBR).
// Ref: https://google.github.io/filament/Filament.md.html#materialsystem/standardmodelsummary
//
// ===============================================================================================

#[inline]
fn clamp(x: f64, xmin: f64, xmax: f64) -> f64 {
    x.clamp(xmin, xmax)
}

#[inline]
fn exp2(x: f64) -> f64 {
    2.0_f64.powf(x)
}

#[inline]
fn max(x: f64, y: f64) -> f64 {
    x.max(y)
}

#[inline]
fn min(x: f64, y: f64) -> f64 {
    x.min(y)
}

#[inline]
fn powf(lhs: f64, rhs: f64) -> f64 {
    lhs.powf(rhs)
}

#[inline]
fn sqrt(x: f64) -> f64 {
    x.sqrt()
}

const PI: f64 = std::f64::consts::PI;

#[inline]
fn D_GGX(NoH: f64, a: f64) -> f64 {
    let a2 = a * a;
    let f = (NoH * a2 - NoH) * NoH + 1.0;
    return a2 / (PI * f * f);
}

#[inline]
fn F_Schlick(u: f64, f0: Vec3) -> Vec3 {
    return f0 + (Vec3::splat(1.0) - f0) * powf(1.0 - u, 5.0);
}

#[inline]
fn V_SmithGGXCorrelated(NoV: f64, NoL: f64, a: f64) -> f64 {
    let a2 = a * a;
    let GGXL = NoV * sqrt((-NoL * a2 + NoL) * NoL + a2);
    let GGXV = NoL * sqrt((-NoV * a2 + NoV) * NoV + a2);
    return 0.5 / (GGXV + GGXL);
}

const fn Fd_Lambert() -> f64 {
    return 1.0 / PI;
}

fn BRDF(
    l: Vec3,
    n: Vec3,
    v: Vec3,
    NoL: f64,
    NoV: f64,
    diffuseColor: Vec3,
    f0: Vec3,
    roughness: f64,
) -> Vec3 {
    let h = Vec3::normalize(v + l);

    let NoH = clamp(Vec3::dot(&n, &h), 0.0, 1.0);
    let LoH = clamp(Vec3::dot(&l, &h), 0.0, 1.0);

    let D = D_GGX(NoH, roughness);
    let F = F_Schlick(LoH, f0);
    let V = V_SmithGGXCorrelated(NoV, NoL, roughness);

    // Specular BRDF.
    let Fr = (D * V) * F;

    // Diffuse BRDF.
    let Fd = diffuseColor * Fd_Lambert();

    return Fr + Fd;
}

// Ref: https://www.unrealengine.com/en-US/blog/physically-based-shading-on-mobile
fn EnvBRDFApprox(colour: Vec3, NoV: f64, perceptual_roughness: f64) -> Vec3 {
    const C0: [f64; 4] = [-1.0, -0.0275, -0.572, 0.022];
    const C1: [f64; 4] = [1.0, 0.0425, 1.04, -0.04];
    let r = [
        perceptual_roughness * C0[0] + C1[0],
        perceptual_roughness * C0[1] + C1[1],
        perceptual_roughness * C0[2] + C1[2],
        perceptual_roughness * C0[3] + C1[3],
    ];
    let a004 = min(r[0] * r[0], exp2(-9.28 * NoV)) * r[0] + r[1];
    let x = -1.04 * a004 + r[2];
    let y = 1.04 * a004 + r[3];
    return x * colour + y;
}
