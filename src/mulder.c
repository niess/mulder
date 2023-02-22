/* C standard library */
#include <float.h>
#include <math.h>
#include <stddef.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>

/* Custom libraries */
#include "mulder.h"
#include "pumas.h"
#include "turtle.h"


/* Height of bottom layer */
#define ZMIN -11E+03

/* Top most height */
#define ZMAX 120E+03

/* Muon rest mass, in GeV / c^2 */
#define MUON_MASS 0.10566

/* Muon decay length, in m */
#define MUON_C_TAU 658.654

#ifndef M_PI
/* Define pi, if unknown */
#define M_PI 3.14159265358979323846
#endif


/* Default error handler */
static void default_error(const char * message)
{
        fputs("error[mulder]: ", stderr);
        fputs(message, stderr);
        fputs("\n", stderr);
        exit(EXIT_FAILURE);
}

void (*mulder_error)(const char * message) = &default_error;


/* Pumas & Turtle error handlers */
static void pumas_error(
    enum pumas_return rc, pumas_function_t * caller, const char * message)
{
        mulder_error(message);
}

static pumas_handler_cb * pumas_default_error = NULL;

static void turtle_error(enum turtle_return code, turtle_function_t * function,
    const char * message)
{
        mulder_error(message);
}

static turtle_error_handler_t * turtle_default_error = NULL;


/* Library initialisation (automatic when loaded, assuming gcc, clang, etc.) */
#ifndef _MULDER_CONSTRUCTOR
__attribute__((__constructor__)) static
#endif
void mulder_initialise(void)
{
        /* Redirect Pumas & Turtle error handlers */
        pumas_default_error = pumas_error_handler_get();
        pumas_error_handler_set(&pumas_error);
        turtle_default_error = turtle_error_handler_get();
        turtle_error_handler_set(&turtle_error);
}


/* Library finalisation (automatic when unloaded, assuming gcc, clang, etc.) */
#ifndef _MULDER_DESTRUCTOR
__attribute__((__destructor__)) static
#endif
void mulder_finalise(void)
{
        /* Restore Pumas & Turtle error handlers */
        pumas_error_handler_set(pumas_default_error);
        pumas_default_error = NULL;
        turtle_error_handler_set(turtle_default_error);
        turtle_default_error = NULL;
}


/* Utility functions for initialising non mutable properties */
static void init_string(void ** loc, const char * s)
{
        size_t size = strlen(s) + 1;
        *loc = malloc(size);
        memcpy(*loc, s, size);
}


static void init_double(double * loc, double d)
{
        *loc = d;
}


static void init_int(int * loc, int i)
{
        *loc = i;
}


static void init_ptr(void ** loc, void * ptr)
{
        *loc = ptr;
}


/* Internal data layout of a topography layer */
struct layer {
        struct mulder_layer api;
        struct turtle_map * map;
};


struct mulder_layer * mulder_layer_create(
    const char * material,
    const char * model,
    double offset)
{
        struct layer * layer = malloc(sizeof *layer);

        /* Load the map */
        if (model == NULL) {
                layer->map = NULL;
                init_ptr((void **)&layer->api.model, NULL);

                /* Zero map metadata */
                init_ptr((void **)&layer->api.encoding, NULL);
                init_ptr((void **)&layer->api.projection, NULL);
                init_int((int *)&layer->api.nx, 0);
                init_int((int *)&layer->api.ny, 0);
                init_double((double *)&layer->api.xmin, -180.);
                init_double((double *)&layer->api.xmax, 180.);
                init_double((double *)&layer->api.ymin, -90.);
                init_double((double *)&layer->api.ymax, 90.);
                init_double((double *)&layer->api.zmin, offset);
                init_double((double *)&layer->api.zmax, offset);
        } else {
                if (turtle_map_load(&layer->map, model) !=
                    TURTLE_RETURN_SUCCESS) {
                        free(layer);
                        return NULL;
                }
                init_string((void **)&layer->api.model, model);

                /* Fetch map metadata */
                struct turtle_map_info info;
                const char * projection;
                turtle_map_meta(layer->map, &info, &projection);

                init_string((void **)&layer->api.encoding, info.encoding);
                init_string((void **)&layer->api.projection, projection);
                init_int((int *)&layer->api.nx, info.nx);
                init_int((int *)&layer->api.ny, info.ny);
                init_double((double *)&layer->api.xmin, info.x[0]);
                init_double((double *)&layer->api.xmax, info.x[1]);
                init_double((double *)&layer->api.ymin, info.y[0]);
                init_double((double *)&layer->api.ymax, info.y[1]);
                init_double((double *)&layer->api.zmin, info.z[0] + offset);
                init_double((double *)&layer->api.zmax, info.z[1] + offset);
        }

        /* Initialise remaining non mutable settings */
        init_string((void **)&layer->api.material, material);
        init_double((double *)&layer->api.offset, offset);

        /* Initialise mutable propertie(s) */
        layer->api.density = 0.;

        return &layer->api;
}


void mulder_layer_destroy(struct mulder_layer ** layer)
{
        if ((layer == NULL) || (*layer == NULL)) return;

        struct layer * l = (void *)(*layer);
        turtle_map_destroy(&l->map);
        free((void *)l->api.material);
        free((void *)l->api.model);
        free((void *)l->api.encoding);
        free((void *)l->api.projection);
        free(l);
        *layer = NULL;
}


double mulder_layer_height(
    const struct mulder_layer * layer, double x, double y)
{
        struct layer * l = (void *)layer;
        if (l->map == NULL) {
                return layer->offset;
        } else {
                double z;
                int inside;
                turtle_map_elevation(l->map, x, y, &z, &inside);
                return inside ? z + layer->offset : ZMIN;
        }
}


void mulder_layer_gradient(const struct mulder_layer * layer,
    double x, double y, double * gx, double *gy)
{
        struct layer * l = (void *)layer;
        if (l->map == NULL) {
                *gx = *gy = 0;
        } else {
                int inside;
                turtle_map_gradient(l->map, x, y, gx, gy, &inside);
                if (!inside) {
                        *gx = *gy = 0.;
                }
        }
}


void mulder_layer_geodetic(const struct mulder_layer * layer, double x,
    double y, double * latitude, double * longitude)
{
        struct layer * l = (void *)layer;
        if (l->map == NULL) {
                *longitude = x;
                *latitude = y;
        } else {
                const struct turtle_projection * p =
                    turtle_map_projection(l->map);
                turtle_projection_unproject(p, x, y, latitude, longitude);
        }
}


void mulder_layer_coordinates(const struct mulder_layer * layer,
    double latitude, double longitude, double * x, double * y)
{
        struct layer * l = (void *)layer;
        if (l->map == NULL) {
                *x = longitude;
                *y = latitude;
        } else {
                const struct turtle_projection * p =
                    turtle_map_projection(l->map);
                turtle_projection_project(p, latitude, longitude, x, y);
        }
}


/* Internal data layout of a fluxmeter */
struct fluxmeter {
        struct mulder_fluxmeter api;
        struct pumas_physics * physics;
        struct pumas_context * context;
        struct pumas_medium * layers_media;
        struct pumas_medium atmosphere_medium;
        struct turtle_stepper * layers_stepper;
        struct turtle_stepper * opensky_stepper;
        double * grammage;
        double zmax;
        double ztop;
        double zref;
        struct mulder_layer * layers[];
};


static double local_properties(struct pumas_medium * medium,
    struct pumas_state * state, struct pumas_locals * locals);

static enum pumas_step layers_geometry(struct pumas_context * context,
    struct pumas_state * state, struct pumas_medium ** medium, double * step);

static enum pumas_step opensky_geometry(struct pumas_context * context,
    struct pumas_state * state, struct pumas_medium ** medium, double * step);

static double reference_flux(double energy, double elevation);

static void update_steppers(struct fluxmeter * fluxmeter);


struct mulder_fluxmeter * mulder_fluxmeter_create(const char * physics,
    int size, struct mulder_layer * layers[])
{
        /* Allocate memory */
        struct fluxmeter * fluxmeter = malloc(
            sizeof(*fluxmeter) +
            size * sizeof(*fluxmeter->layers) +
            (size + 1) * sizeof(*fluxmeter->layers_media));

        /* Initialise PUMAS related data */
        FILE * fid = fopen(physics, "r");
        if (fid == NULL) {
                /* XXX Do some error */
        }
        pumas_physics_load(&fluxmeter->physics, fid);
        fclose(fid);
        pumas_context_create(&fluxmeter->context, fluxmeter->physics, 0);

        fluxmeter->context->mode.scattering = PUMAS_MODE_DISABLED;
        fluxmeter->context->mode.decay = PUMAS_MODE_DISABLED;

        fluxmeter->layers_media = (void *)fluxmeter->layers +
            size * sizeof(*fluxmeter->layers);

        int i;
        fluxmeter->zmax = -DBL_MAX;
        for (i = 0; i < size; i++) {
                pumas_physics_material_index(fluxmeter->physics,
                    layers[i]->material, &fluxmeter->layers_media[i].material);
                fluxmeter->layers_media[i].locals = &local_properties;
                if (layers[i]->zmax > fluxmeter->zmax) {
                        fluxmeter->zmax = layers[i]->zmax;
                }
        }

        pumas_physics_material_index(fluxmeter->physics,
            "Air", &fluxmeter->atmosphere_medium.material);
        /* XXX Add a density profile for the atmosphere */
        fluxmeter->atmosphere_medium.locals = NULL;

        /* Initialise non-mutable settings */
        init_string((void **)&fluxmeter->api.physics, physics);
        init_int((int *)&fluxmeter->api.size, size);
        init_ptr((void **)&fluxmeter->api.layers, fluxmeter->layers);
        memcpy(fluxmeter->layers, layers, size * sizeof *fluxmeter->layers);

        /* Initialise reference flux */
        fluxmeter->api.reference_height = 0.;
        fluxmeter->api.reference_flux = &reference_flux;

        /* Initialise Turtle stepper(s) */
        fluxmeter->zref = DBL_MAX;
        fluxmeter->layers_stepper = NULL;
        fluxmeter->opensky_stepper = NULL;
        update_steppers(fluxmeter);

        return &fluxmeter->api;
}


void mulder_fluxmeter_destroy(struct mulder_fluxmeter ** fluxmeter)
{
        if ((fluxmeter == NULL) || (*fluxmeter == NULL)) return;
        struct fluxmeter * f = (void *)(*fluxmeter);

        pumas_context_destroy(&f->context);
        pumas_physics_destroy(&f->physics);
        turtle_stepper_destroy(&f->layers_stepper);
        turtle_stepper_destroy(&f->opensky_stepper);

        free((void *)f->api.physics);
        free(f->physics);
        free(f);
        *fluxmeter = NULL;
}


/* Update Turtle steppers for the layered & opensky geometries */
static void update_steppers(struct fluxmeter * fluxmeter)
{
        if (fluxmeter->api.reference_height == fluxmeter->zref) {
                return; /* geometry is already up-to-date */
        } else {
                fluxmeter->zref = fluxmeter->api.reference_height;
        }

        /* Destroy previous steppers */
        turtle_stepper_destroy(&fluxmeter->layers_stepper);
        turtle_stepper_destroy(&fluxmeter->opensky_stepper);

        /* Create stepper for the layered geometry */
        turtle_stepper_create(&fluxmeter->layers_stepper);
        turtle_stepper_add_flat(fluxmeter->layers_stepper, ZMIN);

        int i;
        for (i = 0; i < fluxmeter->api.size; i++) {
                turtle_stepper_add_layer(fluxmeter->layers_stepper);
                struct layer * l = (void *)fluxmeter->layers[i];
                if (fluxmeter->layers[i]->model == NULL) {
                        turtle_stepper_add_flat(
                            fluxmeter->layers_stepper, l->api.offset);
                } else {
                        turtle_stepper_add_map(
                            fluxmeter->layers_stepper, l->map, l->api.offset);
                }
        }

        fluxmeter->ztop =
            (fluxmeter->zmax > fluxmeter->api.reference_height) ?
            fluxmeter->zmax : fluxmeter->api.reference_height;
        turtle_stepper_add_layer(fluxmeter->layers_stepper);
        turtle_stepper_add_flat(fluxmeter->layers_stepper, fluxmeter->ztop);

        turtle_stepper_add_layer(fluxmeter->layers_stepper);
        turtle_stepper_add_flat(fluxmeter->layers_stepper, ZMAX);

        /* Create stepper for the opensky geometry */
        turtle_stepper_create(&fluxmeter->opensky_stepper);
        turtle_stepper_add_flat(fluxmeter->opensky_stepper,
            fluxmeter->api.reference_height);

        turtle_stepper_add_layer(fluxmeter->opensky_stepper);
        turtle_stepper_add_flat(fluxmeter->opensky_stepper, ZMAX);
}


/* Wrapper for a pumas Monte Carlo state */
struct state {
        struct pumas_state api;
        struct fluxmeter * fluxmeter;
};


/* Compute muon flux */
double mulder_fluxmeter_flux(
    struct mulder_fluxmeter * fluxmeter, double kinetic_energy, double latitude,
    double longitude, double height, double azimuth, double elevation)
{
        /* Initialise the muon state */
        struct fluxmeter * f = (void *)fluxmeter;
        struct state s = {
                .api = {
                        .charge = 1.,
                        .energy = kinetic_energy,
                        .weight = 1.
                },
                .fluxmeter = f
        };

        turtle_ecef_from_geodetic(latitude, longitude, height, s.api.position);
        turtle_ecef_from_horizontal(
            latitude, longitude, azimuth, elevation, s.api.direction);

        int i;
        for (i = 0; i < 3; i++) {
                /* Revert direction, due to observer convention */
                s.api.direction[i] = -s.api.direction[i];
        }

        /* Update Turtle steppers (if the reference height has changed) */
        update_steppers(f);

        f->context->mode.energy_loss = PUMAS_MODE_CSDA;
        f->context->event = PUMAS_EVENT_LIMIT_ENERGY;

        if (height < f->ztop - FLT_EPSILON) {
                /* Transport backward with Pumas */
                f->context->medium = &layers_geometry;
                f->context->mode.direction = PUMAS_MODE_BACKWARD;
                f->context->limit.energy = 1E+21;

                enum pumas_event event;
                pumas_context_transport(f->context, &s.api, &event, NULL);
                if (event != PUMAS_EVENT_MEDIUM) return 0.;

                /* Get coordinates at end location (expected to be at ztop) */
                turtle_ecef_to_geodetic(
                    s.api.position, &latitude, &longitude, &height);
                if (fabs(height - f->ztop) > 1E-04) return 0.;
        }

        if (height > f->api.reference_height + FLT_EPSILON) {
                /* Backup proper time and kinetic energy */
                const double t0 = s.api.time;
                const double e0 = s.api.energy;
                s.api.time = 0.;

                /* Transport forward to reference height */
                f->context->medium = &opensky_geometry;
                f->context->mode.direction = PUMAS_MODE_FORWARD;
                f->context->limit.energy = 1E-04;

                enum pumas_event event;
                pumas_context_transport(f->context, &s.api, &event, NULL);
                if (event != PUMAS_EVENT_MEDIUM) return 0.;

                /* Get coordinates at end location (expected to be at zref) */
                turtle_ecef_to_geodetic(
                    s.api.position, &latitude, &longitude, &height);
                if (fabs(height - f->api.reference_height) > 1E-04) return 0.;

                /* Update proper time and Jacobian weight */
                s.api.time = t0 - s.api.time;

                const int material = f->atmosphere_medium.material;
                double dedx0, dedx1;
                pumas_physics_property_stopping_power(f->physics,
                    PUMAS_MODE_CSDA, material, e0, &dedx0);
                pumas_physics_property_stopping_power(f->physics,
                    PUMAS_MODE_CSDA, material, s.api.energy, &dedx1);
                if ((dedx0 <= 0.) || (dedx1 <= 0.)) return 0.;
                s.api.weight *= dedx1 / dedx0;
        }

        /* Get direction at reference height */
        const double direction0[] = {
            -s.api.direction[0], -s.api.direction[1], -s.api.direction[2]
        };
        turtle_ecef_to_horizontal(
            latitude, longitude, direction0, &azimuth, &elevation);

        /* Sample flux at reference height */
        const double flux = fluxmeter->reference_flux(
            s.api.energy, elevation);

        /* Compute decay probaility */
        const double pdec = exp(-s.api.time / MUON_C_TAU);

        return flux * pdec * s.api.weight;
}


/* Compute first intersection with topographic layer(s) */
int mulder_fluxmeter_intersect(
    struct mulder_fluxmeter * fluxmeter, double * latitude,
    double * longitude, double * height, double azimuth, double elevation)
{
        /* Initialise the muon state */
        struct fluxmeter * f = (void *)fluxmeter;
        struct state s = {
                .api = {
                        .charge = 1.,
                        .energy = 1.,
                        .weight = 1.
                },
                .fluxmeter = f
        };

        turtle_ecef_from_geodetic(
            *latitude, *longitude, *height, s.api.position);
        turtle_ecef_from_horizontal(
            *latitude, *longitude, azimuth, elevation, s.api.direction);

        /* Update Turtle steppers (if the reference height has changed) */
        update_steppers(f);

        /* Transport with Pumas */
        f->context->medium = &layers_geometry;
        f->context->mode.direction = PUMAS_MODE_FORWARD;
        f->context->mode.energy_loss = PUMAS_MODE_DISABLED;
        f->context->event = PUMAS_EVENT_MEDIUM;

        enum pumas_event event;
        struct pumas_medium * media[2];
        pumas_context_transport(f->context, &s.api, &event, media);
        if (event != PUMAS_EVENT_MEDIUM) return -1;

        /* Get coordinates at end location */
        turtle_ecef_to_geodetic(
            s.api.position, latitude, longitude, height);

        if (media[1] == NULL) {
                return -1;
        } else if (media[1] == &f->atmosphere_medium) {
                return f->api.size;
        } else {
                ptrdiff_t i = media[1] - f->layers_media;
                return (int)i;
        }
}


/* Compute grammage (a.k.a. column depth) along a line of sight */
double mulder_fluxmeter_grammage(
    struct mulder_fluxmeter * fluxmeter, double latitude, double longitude,
    double height, double azimuth, double elevation, double * grammage)
{
        /* Initialise the muon state */
        struct fluxmeter * f = (void *)fluxmeter;
        struct state s = {
                .api = {
                        .charge = 1.,
                        .energy = 1.,
                        .weight = 1.
                },
                .fluxmeter = f
        };

        turtle_ecef_from_geodetic(
            latitude, longitude, height, s.api.position);
        turtle_ecef_from_horizontal(
            latitude, longitude, azimuth, elevation, s.api.direction);

        /* Update Turtle steppers (if the reference height has changed) */
        update_steppers(f);

        /* Transport with Pumas */
        f->context->medium = &layers_geometry;
        f->context->mode.direction = PUMAS_MODE_FORWARD;
        f->context->mode.energy_loss = PUMAS_MODE_DISABLED;
        f->context->event = PUMAS_EVENT_MEDIUM;

        if (grammage != NULL) {
                memset(grammage, 0x0, (f->api.size + 1) * sizeof(*grammage));
        }

        double last_grammage = 0.;
        for (;;) {
                enum pumas_event event;
                struct pumas_medium * media[2];
                pumas_context_transport(f->context, &s.api, &event, media);

                if (grammage != NULL) {
                        int i;
                        if (media[0] == NULL) {
                                break;
                        } else if (media[0] == &f->atmosphere_medium) {
                                i = f->api.size;
                        } else {
                                ptrdiff_t tmp = media[0] - f->layers_media;
                                i = (int)tmp;
                        }
                        grammage[i] += s.api.grammage - last_grammage;
                        last_grammage = s.api.grammage;
                }

                if ((event != PUMAS_EVENT_MEDIUM) || (media[1] == NULL)) break;
        }

        return s.api.grammage;
}


/* Geometry layer index for the given location */
int mulder_fluxmeter_whereami(struct mulder_fluxmeter * fluxmeter,
    double latitude, double longitude, double height)
{
        struct fluxmeter * f = (void *)fluxmeter;

        /* Update Turtle steppers (if the reference height has changed) */
        update_steppers(f);

        /* Get ECEF position */
        double position[3];
        turtle_ecef_from_geodetic(latitude, longitude, height, position);

        /* Fetch location */
        int index[2];
        turtle_stepper_step(f->layers_stepper, position, NULL, NULL,
            NULL, NULL, NULL, NULL, index);

        return (index[0] > 0) ? index[0] - 1 : -1;
}


/* Callback for setting local properties of pumas media */
static double local_properties(struct pumas_medium * medium,
    struct pumas_state * state, struct pumas_locals * locals)
{
        struct state * s = (void *)state;
        struct fluxmeter * f = s->fluxmeter;
        ptrdiff_t i = medium - f->layers_media; /* XXX check sign? */
        locals->density = f->layers[i]->density;
        return 0.;
}


/* Pumas locator for the layered geometry */
static enum pumas_step layers_geometry(struct pumas_context * context,
    struct pumas_state * state, struct pumas_medium ** medium_ptr,
    double * step_ptr)
{
        struct state * s = (void *)state;
        struct fluxmeter * f = s->fluxmeter;

        double step;
        int index[2];
        turtle_stepper_step(f->layers_stepper, state->position, NULL, NULL,
            NULL, NULL, NULL, &step, index);

        if (step_ptr != NULL) {
                *step_ptr = (step <= FLT_EPSILON) ? FLT_EPSILON : step;
        }

        if (medium_ptr != NULL) {
                if ((index[0] >= 1) && (index[0] <= f->api.size)) {
                        *medium_ptr = f->layers_media + (index[0] - 1);
                } else if (index[0] == f->api.size + 1) {
                        *medium_ptr = &f->atmosphere_medium;
                } else if ((context->mode.direction == PUMAS_MODE_FORWARD) &&
                           (index[0] == f->api.size + 2)) {
                        /* Intersect case */
                        *medium_ptr = &f->atmosphere_medium;
                } else {
                        *medium_ptr = NULL;
                }
        }

        return PUMAS_STEP_CHECK;
}


/* Pumas locator for the opensky geometry */
static enum pumas_step opensky_geometry(struct pumas_context * context,
    struct pumas_state * state, struct pumas_medium ** medium_ptr,
    double * step_ptr)
{
        struct state * s = (void *)state;
        struct fluxmeter * f = s->fluxmeter;

        double step;
        int index[2];
        turtle_stepper_step(f->opensky_stepper, state->position, NULL, NULL,
            NULL, NULL, NULL, &step, index);

        if (step_ptr != NULL) {
                *step_ptr = (step <= FLT_EPSILON) ? FLT_EPSILON : step;
        }

        if (medium_ptr != NULL) {
                if (index[0] == 1) {
                        *medium_ptr = &f->atmosphere_medium;
                } else {
                        *medium_ptr = NULL;
                }
        }

        return PUMAS_STEP_CHECK;
}


/* Gaisser's flux model (in GeV^-1 m^-2 s^-1 sr^-1)
 * Ref: see e.g. the ch.30 of the PDG (https://pdglive.lbl.gov)
 */
static double flux_gaisser(double cos_theta, double kinetic_energy)
{
        if (cos_theta < 0.) {
                return 0.;
        } else {
                const double Emu = kinetic_energy + 0.10566;
                const double ec = 1.1 * Emu * cos_theta;
                const double rpi = 1. + ec / 115.;
                const double rK = 1. + ec / 850.;
                return 1.4E+03 * pow(Emu, -2.7) * (1. / rpi + 0.054 / rK);
        }
}


/* Volkova's parameterization of cos(theta*)
 *
 * This is a correction for the Earth curvature, relevant for close to
 * horizontal trajectories.
 * */
static double cos_theta_star(double cos_theta)
{
        const double p[] = {
            0.102573, -0.068287, 0.958633, 0.0407253, 0.817285};
        const double cs2 =
            (cos_theta * cos_theta + p[0] * p[0] + p[1] * pow(cos_theta, p[2]) +
                p[3] * pow(cos_theta, p[4])) /
            (1. + p[0] * p[0] + p[1] + p[3]);
        return cs2 > 0. ? sqrt(cs2) : 0.;
}


/*
 * Guan et al. parameterization of the sea level flux of atmospheric muons
 * Ref: https://arxiv.org/abs/1509.06176
 */
double flux_gccly(double cos_theta, double kinetic_energy)
{
        const double Emu = kinetic_energy + MUON_MASS;
        const double cs = cos_theta_star(cos_theta);
        return pow(1. + 3.64 / (Emu * pow(cs, 1.29)), -2.7) *
            flux_gaisser(cs, kinetic_energy);
}


/* Reference flux model, in GeV^-1 m^-2 s^-1 sr^-1 */
static double reference_flux(double kinetic_energy, double elevation)
{
        const double deg = M_PI / 180.;
        const double cos_theta = cos((90. - elevation) * deg);
        return flux_gccly(cos_theta, kinetic_energy);
}


/* Floating point exceptions (for debugging, disabled by default) */
#ifdef _ENABLE_FE
#ifndef __USE_GNU
#define __USE_GNU
#endif
#include <fenv.h>

__attribute__((constructor))
static void enable_fe(void)
{
        feclearexcept(FE_ALL_EXCEPT);
        feenableexcept(
            FE_DIVBYZERO | FE_INVALID | FE_OVERFLOW | FE_UNDERFLOW);
}
#endif
