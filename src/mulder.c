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
        double zmax;
        double ztop;
        double zref;
        double zref_min;
        double zref_max;
        int use_external_layer;
        struct mulder_layer * layers[];
};


static double layers_locals(struct pumas_medium * medium,
    struct pumas_state * state, struct pumas_locals * locals);

static double atmosphere_locals(struct pumas_medium * medium,
    struct pumas_state * state, struct pumas_locals * locals);

static enum pumas_step layers_geometry(struct pumas_context * context,
    struct pumas_state * state, struct pumas_medium ** medium, double * step);

static enum pumas_step opensky_geometry(struct pumas_context * context,
    struct pumas_state * state, struct pumas_medium ** medium, double * step);

static void update_steppers(struct fluxmeter * fluxmeter);

static struct mulder_reference default_reference;


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
                free(fluxmeter);
                char tmp[strlen(physics) + 32];
                sprintf(tmp, "could not open physics (%s)", physics);
                mulder_error(tmp);
                return NULL;
        }
        pumas_error_catch(1);
        if (pumas_physics_load(&fluxmeter->physics, fid) !=
            PUMAS_RETURN_SUCCESS) {
                fclose(fid);
                free(fluxmeter);
                pumas_error_raise();
                return NULL;
        }
        fclose(fid);
        pumas_error_catch(0);
        pumas_context_create(&fluxmeter->context, fluxmeter->physics, 0);

        fluxmeter->context->mode.scattering = PUMAS_MODE_DISABLED;
        fluxmeter->context->mode.decay = PUMAS_MODE_DISABLED;

        fluxmeter->layers_media = (void *)fluxmeter->layers +
            size * sizeof(*fluxmeter->layers);

        int i;
        fluxmeter->zmax = -DBL_MAX;
        for (i = 0; i < size; i++) {
                pumas_error_catch(1);
                if (pumas_physics_material_index(fluxmeter->physics,
                    layers[i]->material, &fluxmeter->layers_media[i].material)
                    != PUMAS_RETURN_SUCCESS) {
                        free(fluxmeter);
                        pumas_error_raise();
                        return NULL;
                }
                pumas_error_catch(0);
                fluxmeter->layers_media[i].locals = &layers_locals;
                if (layers[i]->zmax > fluxmeter->zmax) {
                        fluxmeter->zmax = layers[i]->zmax;
                }
        }

        pumas_error_catch(1);
        if (pumas_physics_material_index(fluxmeter->physics,
            "Air", &fluxmeter->atmosphere_medium.material) !=
            PUMAS_RETURN_SUCCESS) {
                free(fluxmeter);
                pumas_error_raise();
                return NULL;
        }
        pumas_error_catch(0);
        fluxmeter->atmosphere_medium.locals = &atmosphere_locals;

        /* Initialise non-mutable settings */
        init_string((void **)&fluxmeter->api.physics, physics);
        init_int((int *)&fluxmeter->api.size, size);
        init_ptr((void **)&fluxmeter->api.layers, fluxmeter->layers);
        memcpy(fluxmeter->layers, layers, size * sizeof *fluxmeter->layers);

        /* Initialise transport mode */
        fluxmeter->api.mode = MULDER_CSDA;

        /* Initialise reference flux */
        fluxmeter->api.reference = &default_reference;
        fluxmeter->zref = 0.;
        fluxmeter->zref_min = DBL_MAX;
        fluxmeter->zref_max = -DBL_MAX;

        /* Initialise Turtle stepper(s) */
        fluxmeter->layers_stepper = NULL;
        fluxmeter->opensky_stepper = NULL;
        fluxmeter->use_external_layer = 0;
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
        const struct mulder_reference * const reference =
            fluxmeter->api.reference;
        if ((fluxmeter->zref_min == reference->height_min) &&
            (fluxmeter->zref_max == reference->height_max)) {
                return; /* geometry is already up-to-date */
        } else {
                fluxmeter->zref_min = reference->height_min;
                fluxmeter->zref_max = reference->height_max;
        }
        double zref_min = reference->height_min;
        double zref_max = reference->height_max;
        if (zref_min > zref_max) {
                const double tmp = zref_min;
                zref_min = zref_max;
                zref_max = tmp;
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

        if (fluxmeter->zmax <= zref_min) {
                fluxmeter->ztop = zref_min;
                fluxmeter->zref = zref_min;
        } else if (fluxmeter->zmax <= zref_max) {
                fluxmeter->ztop = fluxmeter->zmax;
                fluxmeter->zref = fluxmeter->zmax;
        } else {
                fluxmeter->ztop = fluxmeter->zmax;
                fluxmeter->zref = zref_max;
        }

        turtle_stepper_add_layer(fluxmeter->layers_stepper);
        turtle_stepper_add_flat(fluxmeter->layers_stepper, fluxmeter->ztop);

        turtle_stepper_add_layer(fluxmeter->layers_stepper);
        turtle_stepper_add_flat(fluxmeter->layers_stepper, ZMAX);

        /* Create stepper for the opensky geometry */
        turtle_stepper_create(&fluxmeter->opensky_stepper);
        turtle_stepper_add_flat(fluxmeter->opensky_stepper, fluxmeter->zref);

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
        if (kinetic_energy <= 0.) {
                mulder_error("bad kinetic energy (0)");
                return 0.;
        }

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

        /* Update Turtle steppers (if the reference heights have changed) */
        update_steppers(f);

        f->context->event = PUMAS_EVENT_LIMIT_ENERGY;
        f->use_external_layer = (height >= f->ztop + FLT_EPSILON);

        if (height < f->ztop - FLT_EPSILON) {
                /* Transport backward with Pumas */
                f->context->limit.energy = f->api.reference->energy_max;
                if (f->api.mode == MULDER_CSDA) {
                        f->context->mode.energy_loss = PUMAS_MODE_CSDA;
                        f->context->mode.scattering = PUMAS_MODE_DISABLED;
                } else if (f->api.mode == MULDER_MIXED) {
                        f->context->mode.energy_loss = PUMAS_MODE_MIXED;
                        f->context->mode.scattering = PUMAS_MODE_DISABLED;
                } else {
                        /* Detailed mode */
                        if (s.api.energy <= 1E+01 - FLT_EPSILON) {
                                f->context->mode.energy_loss =
                                    PUMAS_MODE_STRAGGLED;
                                f->context->mode.scattering =
                                    PUMAS_MODE_MIXED;
                                f->context->limit.energy = 1E+01;
                        } else if (s.api.energy <= 1E+02 - FLT_EPSILON) {
                                f->context->mode.energy_loss = PUMAS_MODE_MIXED;
                                f->context->mode.scattering =
                                    PUMAS_MODE_MIXED;
                                f->context->limit.energy = 1E+02;
                        } else {
                                /* Mixed mode is used */
                                f->context->mode.energy_loss = PUMAS_MODE_MIXED;
                                f->context->mode.scattering =
                                    PUMAS_MODE_DISABLED;
                        }
                }
                f->context->medium = &layers_geometry;
                f->context->mode.direction = PUMAS_MODE_BACKWARD;

                enum pumas_event event;
                for (;;) {
                        if (pumas_context_transport(
                            f->context, &s.api, &event, NULL)
                            != PUMAS_RETURN_SUCCESS) {
                                return 0.;
                        }
                        if ((f->api.mode == MULDER_DETAILED) &&
                            (event == PUMAS_EVENT_LIMIT_ENERGY)) {
                                if (s.api.energy >=
                                    f->api.reference->energy_max -
                                    FLT_EPSILON) {
                                        return 0.;
                                } else if (s.api.energy >=
                                    1E+02 - FLT_EPSILON) {
                                        f->context->mode.energy_loss =
                                            PUMAS_MODE_MIXED;
                                        f->context->mode.scattering =
                                            PUMAS_MODE_DISABLED;
                                        f->context->limit.energy =
                                            f->api.reference->energy_max;
                                        continue;
                                } else {
                                        f->context->mode.energy_loss =
                                            PUMAS_MODE_MIXED;
                                        f->context->mode.scattering =
                                            PUMAS_MODE_MIXED;
                                        f->context->limit.energy = 1E+02;
                                        continue;
                                }
                        } else if (event != PUMAS_EVENT_MEDIUM) {
                                return 0.;
                        } else {
                                break;
                        }
                }

                /* Get coordinates at end location (expected to be at ztop) */
                turtle_ecef_to_geodetic(
                    s.api.position, &latitude, &longitude, &height);
                if (fabs(height - f->ztop) > 1E-04) return 0.;
        }

        if (height > f->api.reference->height_max + FLT_EPSILON) {
                /* Backup proper time and kinetic energy */
                const double t0 = s.api.time;
                const double e0 = s.api.energy;
                s.api.time = 0.;

                /* Transport forward to reference height using CSDA */
                f->context->mode.energy_loss = PUMAS_MODE_CSDA;
                f->context->mode.scattering = PUMAS_MODE_DISABLED;
                f->context->medium = &opensky_geometry;
                f->context->mode.direction = PUMAS_MODE_FORWARD;
                f->context->limit.energy = f->api.reference->energy_min;

                enum pumas_event event;
                if (pumas_context_transport(f->context, &s.api, &event, NULL)
                    != PUMAS_RETURN_SUCCESS) {
                        return 0.;
                }
                if (event != PUMAS_EVENT_MEDIUM) return 0.;

                /* Get coordinates at end location (expected to be at zref) */
                turtle_ecef_to_geodetic(
                    s.api.position, &latitude, &longitude, &height);
                if (fabs(height - f->zref) > 1E-04) return 0.;

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

        /* Sample reference flux at final height */
        struct mulder_reference * reference = fluxmeter->reference;
        const double flux = reference->flux(
            reference, height, elevation, s.api.energy);

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

        /* Update Turtle steppers (if the reference heights have changed) */
        update_steppers(f);

        /* Transport with Pumas */
        f->context->medium = &layers_geometry;
        f->context->mode.direction = PUMAS_MODE_FORWARD;
        f->context->mode.energy_loss = PUMAS_MODE_DISABLED;
        f->context->event = PUMAS_EVENT_MEDIUM;
        f->use_external_layer = (*height >= f->ztop + FLT_EPSILON);

        enum pumas_event event;
        struct pumas_medium * media[2];
        if (pumas_context_transport(f->context, &s.api, &event, media)
            != PUMAS_RETURN_SUCCESS) {
                return -1;
        }
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

        /* Update Turtle steppers (if the reference heights have changed) */
        update_steppers(f);

        /* Transport with Pumas */
        f->context->medium = &layers_geometry;
        f->context->mode.direction = PUMAS_MODE_FORWARD;
        f->context->mode.energy_loss = PUMAS_MODE_DISABLED;
        f->context->event = PUMAS_EVENT_MEDIUM;
        f->use_external_layer = (height >= f->ztop + FLT_EPSILON);

        if (grammage != NULL) {
                memset(grammage, 0x0, (f->api.size + 1) * sizeof(*grammage));
        }

        double last_grammage = 0.;
        for (;;) {
                enum pumas_event event;
                struct pumas_medium * media[2];
                if (pumas_context_transport(f->context, &s.api, &event, media)
                    != PUMAS_RETURN_SUCCESS) {
                        return 0.;
                }

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

        /* Update Turtle steppers (if the reference heights have changed) */
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


/* Callback for setting local properties of layers media */
static double layers_locals(struct pumas_medium * medium,
    struct pumas_state * state, struct pumas_locals * locals)
{
        struct state * s = (void *)state;
        struct fluxmeter * f = s->fluxmeter;
        ptrdiff_t i = medium - f->layers_media;
        if (i >= 0) {
                locals->density = f->layers[i]->density;
        } else {
                const char format[] = "bad medium index (%d)";
                char msg[sizeof(format) + 16];
                sprintf(msg, format, (int)i);
                mulder_error(msg);
        }
        return 0.;
}


/* CORSIKA parameterisation of the US standard atmospheric density */
static double us_standard_function(double height, double lambda, double b)
{
        return 1E+01 * b / lambda * exp(-height / lambda);
}

static double us_standard_density(double height, double * lambda)
{
        const double hc[4] = { 4.E+03, 1.E+04, 4.E+04, 1.E+05 };
        const double bi[4] = { 1222.6562E+00, 1144.9069E+00, 1305.5948E+00,
                540.1778E+00 };
        const double ci[4] = { 994186.38E+00, 878153.55E+00, 636143.04E+00,
                772170.16E+00 };

        /* Compute the local density */
        int i;
        for (i = 0; i < 4; i++) {
                if (height < hc[i]) {
                        const double lb = ci[i] * 1E-02;
                        *lambda = lb;
                        return us_standard_function(height, lb, bi[i]);
                }
        }

        *lambda = ci[3] * 1E-02;
        return us_standard_function(hc[3], ci[3] * 1E-02, bi[3]);
}


/* Callback for setting local properties of the atmosphere*/
static double atmosphere_locals(struct pumas_medium * medium,
    struct pumas_state * state, struct pumas_locals * locals)
{
        double latitude, longitude, height;
        turtle_ecef_to_geodetic(
            state->position, &latitude, &longitude, &height);
        double lambda;
        locals->density = us_standard_density(height, &lambda);

        double azimuth, elevation;
        turtle_ecef_to_horizontal(latitude, longitude, state->direction,
            &azimuth, &elevation);
        double c = fabs(sin(elevation * M_PI / 180));
        if (c < 0.1) c = 0.1;

        return lambda / c;
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
                } else if ((f->use_external_layer) &&
                           (index[0] == f->api.size + 2)) {
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
static double flux_gccly(double cos_theta, double kinetic_energy)
{
        const double Emu = kinetic_energy + MUON_MASS;
        const double cs = cos_theta_star(cos_theta);
        return pow(1. + 3.64 / (Emu * pow(cs, 1.29)), -2.7) *
            flux_gaisser(cs, kinetic_energy);
}


/* Default reference flux model, in GeV^-1 m^-2 s^-1 sr^-1 */
static double reference_flux(struct mulder_reference * reference,
    double height, double elevation, double kinetic_energy)
{
        const double deg = M_PI / 180.;
        const double cos_theta = cos((90. - elevation) * deg);
        return flux_gccly(cos_theta, kinetic_energy);
}

static struct mulder_reference default_reference = {
        .energy_min = 1E-04,
        .energy_max = 1E+21,
        .height_min = 0.,
        .height_max = 0.,
        .flux = &reference_flux
};

struct mulder_reference * mulder_reference_default(void)
{
        return &default_reference;
}


/* Data layout for a tabulated reference flux */
struct reference_table {
        struct mulder_reference api;
        int n_k;
        int n_c;
        int n_h;
        double k_min;
        double k_max;
        double c_min;
        double c_max;
        double h_min;
        double h_max;
        float data[];
};


/* Intepolation of tabulated reference flux */
static double reference_table_flux(struct mulder_reference * reference,
    double height, double elevation, double kinetic_energy)
{
        struct reference_table * table = (void *)reference;

        /* Compute the interpolation indices and coefficients */
        const double dlk = log(table->k_max / table->k_min) /
            (table->n_k - 1);
        double hk = log(kinetic_energy / table->k_min) / dlk;
        if ((hk < 0.) || (hk > table->n_k - 1)) return 0.;
        const int ik = (int)hk;
        hk -= ik;

        const double deg = M_PI / 180;
        const double c = cos((90 - elevation) * deg);
        const double dc = (table->c_max - table->c_min) / (table->n_c - 1);
        double hc = (c - table->c_min) / dc;
        if ((hc < 0.) || (hc > table->n_c - 1)) return 0.;
        const int ic = (int)hc;
        hc -= ic;

        int ih;
        double hh;
        if (table->n_h > 1) {
                const double dh = (table->h_max - table->h_min) /
                    (table->n_h - 1);
                hh = (height - table->h_min) / dh;
                if ((hh < 0.) || (hh > table->n_h - 1)) return 0.;
                ih = (int)hh;
                hh -= ih;
        } else {
                hh = 0.;
                ih = 0;
        }

        const int ik1 = (ik < table->n_k - 1) ?
            ik + 1 : table->n_k - 1;
        const int ic1 = (ic < table->n_c - 1) ?
            ic + 1 : table->n_c - 1;
        const int ih1 = (ih < table->n_h - 1) ?
            ih + 1 : table->n_h - 1;
        const float * const f000 =
            table->data + 2 * ((ih * table->n_c + ic) *
            table->n_k + ik);
        const float * const f010 =
            table->data + 2 * ((ih * table->n_c + ic1) *
            table->n_k + ik);
        const float * const f100 =
            table->data + 2 * ((ih * table->n_c + ic) *
            table->n_k + ik1);
        const float * const f110 =
            table->data + 2 * ((ih * table->n_c + ic1) *
            table->n_k + ik1);
        const float * const f001 =
            table->data + 2 * ((ih1 * table->n_c + ic) *
            table->n_k + ik);
        const float * const f011 =
            table->data + 2 * ((ih1 * table->n_c + ic1) *
            table->n_k + ik);
        const float * const f101 =
            table->data + 2 * ((ih1 * table->n_c + ic) *
            table->n_k + ik1);
        const float * const f111 =
            table->data + 2 * ((ih1 * table->n_c + ic1) *
            table->n_k + ik1);

        /* Interpolate the flux */
        double flux = 0.;
        int i;
        for (i = 0; i < 2; i++) {
                /* Linear interpolation along cos(theta) */
                const double g00 = f000[i] * (1. - hc) + f010[i] * hc;
                const double g10 = f100[i] * (1. - hc) + f110[i] * hc;
                const double g01 = f001[i] * (1. - hc) + f011[i] * hc;
                const double g11 = f101[i] * (1. - hc) + f111[i] * hc;

                /* Log or linear interpolation along log(kinetic) */
                double g0;
                if ((g00 <= 0.) || (g10 <= 0.))
                        g0 = g00 * (1. - hk) + g10 * hk;
                else
                        g0 = exp(log(g00) * (1. - hk) + log(g10) * hk);

                double g1;
                if ((g01 <= 0.) || (g11 <= 0.))
                        g1 = g01 * (1. - hk) + g11 * hk;
                else
                        g1 = exp(log(g01) * (1. - hk) + log(g11) * hk);

                /* Log or linear interpolation along altitude */
                if ((g0 <= 0.) || (g1 <= 0.))
                        flux += g0 * (1. - hh) + g1 * hh;
                else
                        flux += exp(log(g0) * (1. - hh) + log(g1) * hh);
        }
        return flux;
}


/* Load a tabulated reference flux */
struct mulder_reference * mulder_reference_load_table(const char * path)
{
        FILE * fid = fopen(path, "rb");
        if (fid == NULL) {
                char format[] = "could not open %s";
                const int n = strlen(path) + sizeof format;
                char msg[n];
                sprintf(msg, path);
                mulder_error(msg);
                return NULL;
        }

        struct reference_table * table = NULL;
        int64_t shape[3];
        double range[6];

        if (fread(shape, 8, 3, fid) != 3) goto error;
        if (fread(range, 8, 6, fid) != 6) goto error;

        /* XXX check endianess ? */

        size_t size = (size_t)(2 * shape[0] * shape[1] * shape[2]);
        table = malloc(sizeof(*table) + size * sizeof(*table->data));
        if (table == NULL) goto error;

        if (fread(table->data, sizeof(*table->data), size, fid) != size) {
                goto error;
        }
        fclose(fid);

        table->n_k = shape[0];
        table->n_c = shape[1];
        table->n_h = shape[2];
        table->k_min = range[0];
        table->k_max = range[1];
        table->c_min = range[2];
        table->c_max = range[3];
        table->h_min = range[4];
        table->h_max = range[5];

        /* Set API fields */
        table->api.energy_min = table->k_min;
        table->api.energy_max = table->k_max;
        table->api.height_min = table->h_min;
        table->api.height_max = table->h_max;
        table->api.flux = &reference_table_flux;

        return &table->api;
error:
        fclose(fid);
        free(table);
        char format[] = "bad format (%s)";
        const int n = strlen(path) + sizeof format;
        char msg[n];
        sprintf(msg, path);
        mulder_error(msg);
        return NULL;
}


void mulder_reference_destroy_table(struct mulder_reference ** reference)
{
        if ((reference == NULL) || (*reference == NULL)) return;
        free(*reference);
        *reference = NULL;
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
