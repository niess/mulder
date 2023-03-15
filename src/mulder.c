/* C standard library */
#include <float.h>
#include <math.h>
#include <stddef.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>

/* Custom libraries */
#include "gull.h"
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


/* Formated error */
#define MULDER_ERROR(FORMAT, EXTRA_SIZE, ...)                                  \
{                                                                              \
        const char format[] = FORMAT ;                                         \
        char msg[sizeof(format) + EXTRA_SIZE];                                 \
        sprintf(msg, format, __VA_ARGS__);                                     \
        mulder_error(msg);                                                     \
}


/* Pumas & Turtle error handlers */
static void pumas_error(
    enum pumas_return rc,
    pumas_function_t * caller,
    const char * message)
{
        mulder_error(message);
}

static pumas_handler_cb * pumas_default_error = NULL;

static void turtle_error(
    enum turtle_return code,
    turtle_function_t * function,
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
    const struct mulder_layer * layer,
    const struct mulder_projection projection)
{
        struct layer * l = (void *)layer;
        if (l->map == NULL) {
                return layer->offset;
        } else {
                double z;
                int inside;
                turtle_map_elevation(
                    l->map,
                    projection.x,
                    projection.y,
                    &z,
                    &inside
                );
                return inside ? z + layer->offset : ZMIN;
        }
}


struct mulder_projection mulder_layer_gradient(
    const struct mulder_layer * layer,
    const struct mulder_projection projection)
{
        struct mulder_projection gradient;
        struct layer * l = (void *)layer;
        if (l->map == NULL) {
                gradient.x = gradient.y = 0.;
        } else {
                int inside;
                turtle_map_gradient(
                    l->map,
                    projection.x,
                    projection.y,
                    &gradient.x,
                    &gradient.y,
                    &inside
                );
                if (!inside) {
                        gradient.x = gradient.y = 0.;
                }
        }
        return gradient;
}


struct mulder_position mulder_layer_position(
    const struct mulder_layer * layer,
    const struct mulder_projection projection)
{
        struct mulder_position position;
        struct layer * l = (void *)layer;
        if (l->map == NULL) {
                position.longitude = projection.x;
                position.latitude = projection.y;
        } else {
                const struct turtle_projection * p =
                    turtle_map_projection(l->map);
                turtle_projection_unproject(
                    p,
                    projection.x,
                    projection.y,
                    &position.latitude,
                    &position.longitude
                );
        }

        position.height = mulder_layer_height(layer, projection);
        return position;
}


struct mulder_projection mulder_layer_project(
    const struct mulder_layer * layer,
    const struct mulder_position position)
{
        struct layer * l = (void *)layer;
        struct mulder_projection projection;
        if (l->map == NULL) {
                projection.x = position.longitude;
                projection.y = position.latitude;
        } else {
                const struct turtle_projection * p =
                    turtle_map_projection(l->map);
                turtle_projection_project(
                    p,
                    position.latitude,
                    position.longitude,
                    &projection.x,
                    &projection.y
                );
        }
        return projection;
}


/* Internal data layout of a geomagnetic field */
struct geomagnet {
        struct mulder_geomagnet api;
        struct gull_snapshot * snapshot;
        double * workspace;
};


struct mulder_geomagnet * mulder_geomagnet_create(
    const char * model,
    int day,
    int month,
    int year)
{
        struct geomagnet * geomagnet = malloc(sizeof *geomagnet);

        /* Load the snapshot */
        enum gull_return rc = gull_snapshot_create(
            &geomagnet->snapshot, model, day, month, year);
        if (rc != GULL_RETURN_SUCCESS) {
                free(geomagnet);
                if (rc == GULL_RETURN_MEMORY_ERROR) {
                        mulder_error("could not allocate memory");
                } else if (rc == GULL_RETURN_PATH_ERROR) {
                        MULDER_ERROR("could not open %s", strlen(model), model);
                } else if (rc == GULL_RETURN_MISSING_DATA) {
                        mulder_error("no data for the given date");
                }
                return NULL;
        }

        /* Mirror initial settings */
        init_string((void **)&geomagnet->api.model, model);
        init_int((int *)&geomagnet->api.day, day);
        init_int((int *)&geomagnet->api.month, month);
        init_int((int *)&geomagnet->api.year, year);

        /* Fetch metadata */
        int order;
        double height_min;
        double height_max;
        gull_snapshot_info(
            geomagnet->snapshot, &order, &height_min, &height_max);

        init_int((int *)&geomagnet->api.order, order);
        init_double((double *)&geomagnet->api.height_min, height_min);
        init_double((double *)&geomagnet->api.height_max, height_max);

        /* Initialise workspace */
        geomagnet->workspace = NULL;

        return &geomagnet->api;
}


void mulder_geomagnet_destroy(struct mulder_geomagnet ** geomagnet)
{
        if ((geomagnet == NULL) || (*geomagnet == NULL)) return;

        struct geomagnet * g = (void *)(*geomagnet);
        gull_snapshot_destroy(&g->snapshot);
        free(g->workspace);
        free((void *)g->api.model);
        free(g);
        *geomagnet = NULL;
}


struct mulder_enu mulder_geomagnet_field(
    const struct mulder_geomagnet * geomagnet,
    const struct mulder_position position)
{
        struct geomagnet * g = (void *)geomagnet;
        struct mulder_enu enu;
        enum gull_return rc = gull_snapshot_field(
            g->snapshot,
            position.latitude,
            position.longitude,
            position.height,
            (double *)&enu,
            &g->workspace
        );
        if (rc != GULL_RETURN_SUCCESS) {
                enu.north = enu.east = enu.upward = 0.;
        }
        return enu;
}


/* Geometry create & destroy */
struct mulder_geometry * mulder_geometry_create(
    int size,
    struct mulder_layer * layers[]
)
{
        struct mulder_geometry * geometry = malloc(
            (sizeof *geometry) + size * (sizeof *layers));
        if (geometry == NULL) {
                mulder_error("could not allocate geometry");
                return NULL;
        }

        init_int((int *)&geometry->size, size);
        int i;
        for (i = 0; i < size; i++) {
                init_ptr((void **)&geometry->layers[i], layers[i]);
        }

        return geometry;
}


void mulder_geometry_destroy(struct mulder_geometry ** geometry)
{
        if ((geometry == NULL) || (*geometry == NULL)) {
                return;
        } else {
                free(*geometry);
                *geometry = NULL;
        }
}


/* Internal data layout of a fluxmeter */
struct fluxmeter {
        struct mulder_fluxmeter api;
        struct mulder_prng prng;
        /* Pumas related objects */
        struct pumas_physics * physics;
        struct pumas_context * context;
        double (*context_random)(struct pumas_context * context);
        /* Steppers related data */
        struct turtle_stepper * layers_stepper;
        struct turtle_stepper * opensky_stepper;
        double zmax;
        double ztop;
        double zref;
        double zref_min;
        double zref_max;
        int use_external_layer;
        /* Geomagnet related data */
        struct geomagnet * current_geomagnet;
        double * geomagnet_workspace;
        double geomagnet_field[3];
        double geomagnet_position[3];
        int use_geomagnet;
        /* Layers data */
        struct pumas_medium atmosphere_medium;
        struct pumas_medium layers_media[];
};


/* Prototypes of local functions & data for the fluxmeter implementation */
static double layers_locals(
    struct pumas_medium * medium,
    struct pumas_state * state,
    struct pumas_locals * locals
);

static double atmosphere_locals(
    struct pumas_medium * medium,
    struct pumas_state * state,
    struct pumas_locals * locals
);

static enum pumas_step layers_geometry(
    struct pumas_context * context,
    struct pumas_state * state,
    struct pumas_medium ** medium,
    double * step
);

static enum pumas_step opensky_geometry(
    struct pumas_context * context,
    struct pumas_state * state,
    struct pumas_medium ** medium,
    double * step
);

static void update_steppers(struct fluxmeter * fluxmeter);

static double random_pumas(struct pumas_context * context);

static unsigned long get_seed(struct mulder_prng * prng);

static void set_seed(
    struct mulder_prng * prng,
    const unsigned long * seed
);

static double uniform01(struct mulder_prng * prng);

static struct mulder_reference default_reference;


/* Librray entry point for creating a fluxmeter */
struct mulder_fluxmeter * mulder_fluxmeter_create(
    const char * physics,
    struct mulder_geometry * geometry)
{
        /* Allocate memory */
        const int size = geometry->size;
        struct fluxmeter * fluxmeter = malloc(
            (sizeof *fluxmeter) +
            (size + 1) * (sizeof *fluxmeter->layers_media)
        );

        /* Initialise PUMAS related data */
        FILE * fid = fopen(physics, "r");
        if (fid == NULL) {
                free(fluxmeter);
                MULDER_ERROR(
                    "could not open physics (%s)",
                    strlen(physics),
                    physics
                );
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

        fluxmeter->context->user_data = fluxmeter;
        fluxmeter->context_random = fluxmeter->context->random;
        fluxmeter->context->random = &random_pumas;
        fluxmeter->context->mode.scattering = PUMAS_MODE_DISABLED;
        fluxmeter->context->mode.decay = PUMAS_MODE_DISABLED;

        struct mulder_layer * const * layers = geometry->layers;
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
        init_ptr((void **)&fluxmeter->api.geometry, geometry);

        /* Initialise transport mode etc. */
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

        /* Initialise PRNG API */
        fluxmeter->api.prng = &fluxmeter->prng;
        fluxmeter->prng.get_seed = &get_seed;
        fluxmeter->prng.set_seed = &set_seed;
        fluxmeter->prng.uniform01 = &uniform01;

        /* Initialise geomagnet */
        fluxmeter->current_geomagnet = NULL;
        fluxmeter->geomagnet_workspace = NULL;
        memset(fluxmeter->geomagnet_field, 0x0,
            sizeof(fluxmeter->geomagnet_field));
        memset(fluxmeter->geomagnet_position, 0x0,
            sizeof(fluxmeter->geomagnet_position));
        fluxmeter->use_geomagnet = 0;

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
        free(f->geomagnet_workspace);
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

        struct mulder_geometry * geometry = fluxmeter->api.geometry;
        int i;
        for (i = 0; i < geometry->size; i++) {
                turtle_stepper_add_layer(fluxmeter->layers_stepper);
                struct layer * l = (void *)geometry->layers[i];
                if (l->api.model == NULL) {
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


/* Muon flux computation */
static struct state init_event(
    struct fluxmeter * fluxmeter,
    enum mulder_pid pid,
    struct mulder_position position,
    struct mulder_direction direction,
    double energy
);

static struct mulder_state transport_event(
    struct fluxmeter * fluxmeter,
    struct mulder_position position,
    struct state state
);

struct mulder_flux mulder_fluxmeter_flux(
    struct mulder_fluxmeter * fluxmeter,
    const struct mulder_state initial)
{
        /* Initialise the geometry etc. */
        struct fluxmeter * f = (void *)fluxmeter;
        struct mulder_flux result = {0.};
        struct state s = init_event(
            f,
            MULDER_MUON,
            initial.position,
            initial.direction,
            initial.energy
        );
        if (s.api.weight <= 0.) {
                return result;
        }

        /* Sample the reference flux */
        struct mulder_reference * reference = f->api.reference;
        if (initial.pid == MULDER_ANY) {
                if (f->api.geometry->geomagnet == NULL) {
                        struct mulder_state state =
                            transport_event(f, initial.position, s);
                        if (state.weight <= 0.) {
                                return result;
                        }
                        state.pid = MULDER_ANY;
                        return mulder_state_flux(state, reference);
                } else {
                        s.api.charge = -1.;
                        struct mulder_state s0 =
                            transport_event(f, initial.position, s);
                        struct mulder_flux r0 = mulder_state_flux(
                            s0, reference);

                        s.api.charge = 1.;
                        struct mulder_state s1 =
                            transport_event(f, initial.position, s);
                        struct mulder_flux r1 = mulder_state_flux(
                            s1, reference);

                        const double tmp = r0.value + r1.value;
                        if (tmp > 0.) {
                                result.value = tmp;
                                result.asymmetry = (r1.value - r0.value) / tmp;
                        } else {
                                result.value = 0.;
                                result.asymmetry = 0.;
                        }
                        return result;
                }
        } else {
                s.api.charge = (initial.pid == MULDER_MUON) ?
                    -1. : 1.;
                struct mulder_state state =
                    transport_event(f, initial.position, s);
                if (state.weight <= 0.) {
                        return result;
                }
                return mulder_state_flux(state, reference);
        }
}


/* Monte Carlo interface */
struct mulder_flux mulder_state_flux(
    const struct mulder_state state,
    struct mulder_reference * reference)
{
        struct mulder_flux result = reference->flux(reference,
            state.position.height, state.direction.elevation, state.energy);

        if (state.pid != MULDER_ANY) {
                const double charge = (state.pid == MULDER_MUON) ? -1. : 1.;
                result.value *= 0.5 * (1. + charge * result.asymmetry);
                result.asymmetry = charge;
        }

        result.value *= state.weight;
        return result;
}


struct mulder_state mulder_fluxmeter_transport(
    struct mulder_fluxmeter * fluxmeter,
    const struct mulder_state state)
{
        /* Check pid */
        enum mulder_pid pid = state.pid;
        if ((pid == MULDER_ANY) &&
            (fluxmeter->mode == MULDER_CSDA)) {
                if (fluxmeter->geometry->geomagnet != NULL) {
                        MULDER_ERROR("bad pid (%d)", 16, (int)state.pid);
                        struct mulder_state tmp = {0.};
                        return tmp;
                } else {
                        pid = MULDER_MUON;
                }
        }

        /* Initialise the geometry etc. */
        struct fluxmeter * f = (void *)fluxmeter;
        struct state s = init_event(
            f, pid, state.position, state.direction, state.energy);
        if (s.api.weight <= 0.) {
                struct mulder_state tmp = {0.};
                return tmp;
        }

        /* Transport state */
        struct mulder_state result = transport_event(f, state.position, s);

        /* Restore pid, if needed */
        if ((state.pid == MULDER_ANY) &&
            (fluxmeter->mode == MULDER_CSDA)) {
                result.pid = MULDER_ANY;
        }

        return result;
}


/* Low level sampling routines */
static struct state init_event(
    struct fluxmeter * f,
    enum mulder_pid pid,
    const struct mulder_position position,
    const struct mulder_direction direction,
    double energy)
{
        struct state s = {.api = {.weight = 0.}};
        if (energy <= 0.) {
                MULDER_ERROR("bad kinetic energy (%g)", 32, energy);
                return s;
        }

        /* Update Turtle steppers (if the reference heights have changed) */
        update_steppers(f);

        /* Update geomagnet (if needed) */
        if (f->api.geometry->geomagnet != (void *)f->current_geomagnet) {
                free(f->geomagnet_workspace);
                f->geomagnet_workspace = NULL;
                memset(f->geomagnet_field, 0x0, sizeof f->geomagnet_field);
                memset(f->geomagnet_position, 0x0,
                    sizeof f->geomagnet_position);
                f->current_geomagnet = (void *)f->api.geometry->geomagnet;
        }
        f->use_geomagnet = (f->current_geomagnet != NULL);

        f->context->event = PUMAS_EVENT_LIMIT_ENERGY;
        f->use_external_layer = (position.height >= f->ztop + FLT_EPSILON);

        /* Initialise the muon state */
        s.api.energy = energy;
        s.api.weight = 1.;
        s.fluxmeter = f;

        if (pid == MULDER_ANY) {
                struct mulder_prng * prng = f->api.prng;
                s.api.charge = (prng->uniform01(prng) <= 0.5) ? -1. : 1.;
                s.api.weight *= 2;
        } else {
                s.api.charge = (pid == MULDER_MUON) ? -1. : 1.;
        }

        turtle_ecef_from_geodetic(position.latitude, position.longitude,
            position.height, s.api.position);
        turtle_ecef_from_horizontal(
            position.latitude, position.longitude, direction.azimuth,
            direction.elevation, s.api.direction);

        int i;
        for (i = 0; i < 3; i++) {
                /* Revert direction, due to observer convention */
                s.api.direction[i] = -s.api.direction[i];
        }

        return s;
}

static struct mulder_state transport_event(
    struct fluxmeter * f,
    struct mulder_position position,
    struct state s)
{
        if (position.height < f->ztop - FLT_EPSILON) {
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
                                struct mulder_state state = {0.};
                                return state;
                        }
                        if ((f->api.mode == MULDER_DETAILED) &&
                            (event == PUMAS_EVENT_LIMIT_ENERGY)) {
                                if (s.api.energy >=
                                    f->api.reference->energy_max -
                                    FLT_EPSILON) {
                                        struct mulder_state state = {0.};
                                        return state;
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
                                struct mulder_state state = {0.};
                                return state;
                        } else {
                                break;
                        }
                }

                /* Get coordinates at end location (expected to be at ztop) */
                turtle_ecef_to_geodetic(s.api.position, &position.latitude,
                    &position.longitude, &position.height);
                if (fabs(position.height - f->ztop) > 1E-04) {
                        struct mulder_state state = {0.};
                        return state;
                }
        }

        if (position.height > f->api.reference->height_max + FLT_EPSILON) {
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
                        struct mulder_state state = {0.};
                        return state;
                }
                if (event != PUMAS_EVENT_MEDIUM) {
                        struct mulder_state state = {0.};
                        return state;
                }

                /* Get coordinates at end location (expected to be at zref) */
                turtle_ecef_to_geodetic(s.api.position, &position.latitude,
                    &position.longitude, &position.height);
                if (fabs(position.height - f->zref) > 1E-04) {
                        struct mulder_state state = {0.};
                        return state;
                } else {
                        position.height = f->zref;
                        /* due to potential rounding errors */
                }

                /* Update proper time and Jacobian weight */
                s.api.time = t0 - s.api.time;

                const int material = f->atmosphere_medium.material;
                double dedx0, dedx1;
                pumas_physics_property_stopping_power(f->physics,
                    PUMAS_MODE_CSDA, material, e0, &dedx0);
                pumas_physics_property_stopping_power(f->physics,
                    PUMAS_MODE_CSDA, material, s.api.energy, &dedx1);
                if ((dedx0 <= 0.) || (dedx1 <= 0.)) {
                        struct mulder_state state = {0.};
                        return state;
                }
                s.api.weight *= dedx1 / dedx0;
        }

        /* Get direction at reference height */
        struct mulder_direction direction;
        const double direction0[] = {
            -s.api.direction[0], -s.api.direction[1], -s.api.direction[2]
        };
        turtle_ecef_to_horizontal(position.latitude, position.longitude,
            direction0, &direction.azimuth, &direction.elevation);

        /* Compute decay probability */
        const double pdec = exp(-s.api.time / MUON_C_TAU);

        /* Fill and return the reference state */
        struct mulder_state state = {
                .pid = (s.api.charge < 0.) ? MULDER_MUON : MULDER_ANTIMUON,
                .position = position,
                .direction = direction,
                .energy = s.api.energy,
                .weight = pdec * s.api.weight
        };

        return state;
}


/* Compute first intersection with topographic layer(s) */
struct mulder_intersection mulder_fluxmeter_intersect(
    struct mulder_fluxmeter * fluxmeter,
    const struct mulder_position position,
    const struct mulder_direction direction)
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

        turtle_ecef_from_geodetic(position.latitude, position.longitude,
            position.height, s.api.position);
        turtle_ecef_from_horizontal(position.latitude, position.longitude,
            direction.azimuth, direction.elevation, s.api.direction);

        /* Update Turtle steppers (if the reference heights have changed) */
        update_steppers(f);

        /* Disable geomagnetic field */
        f->use_geomagnet = 0;

        /* Transport with Pumas */
        f->context->medium = &layers_geometry;
        f->context->mode.direction = PUMAS_MODE_FORWARD;
        f->context->mode.energy_loss = PUMAS_MODE_DISABLED;
        f->context->event = PUMAS_EVENT_MEDIUM;
        f->use_external_layer = (position.height >= f->ztop + FLT_EPSILON);

        struct mulder_intersection intersection = {.layer = -1};

        enum pumas_event event;
        struct pumas_medium * media[2];
        if (pumas_context_transport(f->context, &s.api, &event, media)
            != PUMAS_RETURN_SUCCESS) {
                return intersection;
        }
        if (event != PUMAS_EVENT_MEDIUM) return intersection;

        /* Get coordinates at end location */
        turtle_ecef_to_geodetic(
            s.api.position,
            &intersection.position.latitude,
            &intersection.position.longitude,
            &intersection.position.height);

        struct mulder_geometry * geometry = f->api.geometry;
        if (media[1] == NULL) {
                return intersection;
        } else if (media[1] == &f->atmosphere_medium) {
                intersection.layer = geometry->size;
        } else {
                ptrdiff_t i = media[1] - f->layers_media;
                intersection.layer = (int)i;
        }
        return intersection;
}


/* Compute grammage (a.k.a. column depth) along a line of sight */
double mulder_fluxmeter_grammage(
    struct mulder_fluxmeter * fluxmeter,
    const struct mulder_position position,
    const struct mulder_direction direction,
    double * grammage)
{
        /* Initialise the muon state (XXX shared with intersect) */
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
            position.latitude,
            position.longitude,
            position.height,
            s.api.position
        );

        turtle_ecef_from_horizontal(
            position.latitude,
            position.longitude,
            direction.azimuth,
            direction.elevation,
            s.api.direction
        );

        /* Update Turtle steppers (if the reference heights have changed) */
        update_steppers(f);

        /* Disable geomagnetic field */
        f->use_geomagnet = 0;

        /* Transport with Pumas */
        f->context->medium = &layers_geometry;
        f->context->mode.direction = PUMAS_MODE_FORWARD;
        f->context->mode.energy_loss = PUMAS_MODE_DISABLED;
        f->context->event = PUMAS_EVENT_MEDIUM;
        f->use_external_layer = (position.height >= f->ztop + FLT_EPSILON);

        if (grammage != NULL) {
                memset(
                    grammage,
                    0x0,
                    (f->api.geometry->size + 1) * sizeof(*grammage)
                );
        }

        double last_grammage = 0.;
        for (;;) {
                enum pumas_event event;
                struct pumas_medium * media[2];
                enum pumas_return rc = pumas_context_transport(
                    f->context,
                    &s.api,
                    &event,
                    media
                );
                if (rc != PUMAS_RETURN_SUCCESS) {
                        return 0.;
                }

                if (grammage != NULL) {
                        int i;
                        if (media[0] == NULL) {
                                break;
                        } else if (media[0] == &f->atmosphere_medium) {
                                i = f->api.geometry->size;
                        } else {
                                ptrdiff_t tmp = media[0] - f->layers_media;
                                i = (int)tmp;
                        }
                        grammage[i] += s.api.grammage - last_grammage;
                        last_grammage = s.api.grammage;
                }

                if ((event != PUMAS_EVENT_MEDIUM) ||
                    (media[1] == NULL)) {
                        break;
                }
        }

        return s.api.grammage;
}


/* Geometry layer index for the given location */
int mulder_fluxmeter_whereami(
    struct mulder_fluxmeter * fluxmeter,
    const struct mulder_position position)
{
        struct fluxmeter * f = (void *)fluxmeter;

        /* Update Turtle steppers (if the reference heights have changed) */
        update_steppers(f);

        /* Get ECEF position */
        double ecef[3];
        turtle_ecef_from_geodetic(
            position.latitude,
            position.longitude,
            position.height,
            ecef
        );

        /* Fetch location */
        int index[2];
        turtle_stepper_step(
            f->layers_stepper,
            ecef,
            NULL,
            NULL,
            NULL,
            NULL,
            NULL,
            NULL,
            index
        );

        return (index[0] > 0) ? index[0] - 1 : -1;
}


/* Callback for setting local properties of layers media */
static double layers_locals(
    struct pumas_medium * medium,
    struct pumas_state * state,
    struct pumas_locals * locals)
{
        struct state * s = (void *)state;
        struct fluxmeter * f = s->fluxmeter;
        ptrdiff_t i = medium - f->layers_media;
        if (i >= 0) {
                locals->density = f->api.geometry->layers[i]->density;
        } else {
                MULDER_ERROR("bad medium index (%d)", 16, (int)i);
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
        const double hc[4] = {
            4.E+03, 1.E+04, 4.E+04, 1.E+05
        };
        const double bi[4] = {
            1222.6562E+00, 1144.9069E+00, 1305.5948E+00, 540.1778E+00
        };
        const double ci[4] = {
            994186.38E+00, 878153.55E+00, 636143.04E+00, 772170.16E+00
        };

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


/* Compute rotation matrix from ECEF to ENU */
static void ecef_to_enu(
    double latitude,
    double longitude,
    double declination,
    double inclination,
    double rotation[3][3])
{
        turtle_ecef_from_horizontal(
             latitude,
             longitude,
             90. + declination,
             0.,
             &rotation[0][0]
        );

        turtle_ecef_from_horizontal(
            latitude,
            longitude,
            declination,
            -inclination,
            &rotation[1][0]
        );

        turtle_ecef_from_horizontal(
            latitude,
            longitude,
            0.,
            90. - inclination,
            &rotation[2][0]
        );
}


/* Callback for setting local properties of the atmosphere*/
static double atmosphere_locals(
    struct pumas_medium * medium,
    struct pumas_state * state,
    struct pumas_locals * locals)
{
        /* Get local density */
        double latitude, longitude, height;
        turtle_ecef_to_geodetic(
            state->position,
            &latitude,
            &longitude,
            &height
        );
        double lambda;
        locals->density = us_standard_density(height, &lambda);

        double azimuth, elevation;
        turtle_ecef_to_horizontal(
            latitude,
            longitude,
            state->direction,
            &azimuth,
            &elevation
        );
        double c = fabs(sin(elevation * M_PI / 180));
        if (c < 0.1) c = 0.1;
        lambda /= c;

        struct state * s = (void *)state;
        struct fluxmeter * f = s->fluxmeter;
        if (!f->use_geomagnet) {
                return lambda;
        }

        /* Get local geomagnetic field */
        double lambda_g = 1E+03;
        double d2 = 0.;
        int i;
        for (i = 0; i < 3; i++) {
                const double tmp =
                    state->position[i] - f->geomagnet_position[i];
                d2 += tmp * tmp;
        }
        if (d2 > lambda_g * lambda_g) {
                /* Get the local magnetic field (in ENU frame) */
                double enu[3];
                gull_snapshot_field(
                    f->current_geomagnet->snapshot,
                    latitude,
                    longitude,
                    height,
                    enu,
                    &f->geomagnet_workspace
                );

                /* Transform to ECEF (using transposed/inverse matrix) */
                double rotation[3][3];
                ecef_to_enu(
                    latitude,
                    longitude,
                    0.,
                    0.,
                    rotation
                );

                int i;
                double ecef[3] = {0., 0., 0.};
                for (i = 0; i < 3; i++) {
                        int j;
                        for (j = 0; j < 3; j++) {
                                ecef[i] += rotation[j][i] * enu[j];
                        }
                }

                /* Update the cache */
                memcpy(
                    f->geomagnet_field,
                    ecef,
                    sizeof f->geomagnet_field
                );
                memcpy(
                    f->geomagnet_position,
                    state->position,
                    sizeof f->geomagnet_position
                );
        }

        /* Fetch the cached geomagnetic field */
        memcpy(
            locals->magnet,
            f->geomagnet_field,
            sizeof locals->magnet
        );

        lambda_g /= f->context->accuracy;
        return (lambda < lambda_g) ? lambda : lambda_g;
}


/* Pumas locator for the layered geometry */
static enum pumas_step layers_geometry(
    struct pumas_context * context,
    struct pumas_state * state,
    struct pumas_medium ** medium_ptr,
    double * step_ptr)
{
        struct state * s = (void *)state;
        struct fluxmeter * f = s->fluxmeter;

        double step;
        int index[2];
        turtle_stepper_step(
            f->layers_stepper,
            state->position,
            NULL,
            NULL,
            NULL,
            NULL,
            NULL,
            &step,
            index
        );

        if (step_ptr != NULL) {
                *step_ptr = (step <= FLT_EPSILON) ? FLT_EPSILON : step;
        }

        if (medium_ptr != NULL) {
                if ((index[0] >= 1) && (index[0] <= f->api.geometry->size)) {
                        *medium_ptr = f->layers_media + (index[0] - 1);
                } else if (index[0] == f->api.geometry->size + 1) {
                        *medium_ptr = &f->atmosphere_medium;
                } else if ((f->use_external_layer) &&
                           (index[0] == f->api.geometry->size + 2)) {
                        *medium_ptr = &f->atmosphere_medium;
                } else {
                        *medium_ptr = NULL;
                }
        }

        return PUMAS_STEP_CHECK;
}


/* Pumas locator for the opensky geometry */
static enum pumas_step opensky_geometry(
    struct pumas_context * context,
    struct pumas_state * state,
    struct pumas_medium ** medium_ptr,
    double * step_ptr)
{
        struct state * s = (void *)state;
        struct fluxmeter * f = s->fluxmeter;

        double step;
        int index[2];
        turtle_stepper_step(
            f->opensky_stepper,
            state->position,
            NULL,
            NULL,
            NULL,
            NULL,
            NULL,
            &step,
            index
        );

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
                return
                    1.4E+03 *
                    pow(Emu, -2.7) *
                    (1. / rpi + 0.054 / rK)
                ;
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
            (
                cos_theta * cos_theta +
                p[0] * p[0] +
                p[1] * pow(cos_theta, p[2]) +
                p[3] * pow(cos_theta, p[4]))
            /
            (
                1. +
                p[0] * p[0] +
                p[1] +
                p[3]
            )
        ;
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
        return
            pow(1. + 3.64 / (Emu * pow(cs, 1.29)), -2.7) *
            flux_gaisser(cs, kinetic_energy)
        ;
}


/* Fraction of the muon flux for a given charge */
static double charge_fraction(enum mulder_pid pid)
{
        /* Use a constant charge ratio.
         * Ref: CMS (https://arxiv.org/abs/1005.5332)
         */
        const double charge_ratio = 1.2766;

        if (pid == MULDER_MUON) {
                return 1. / (1. + charge_ratio);
        } else if (pid == MULDER_ANTIMUON) {
                return charge_ratio / (1. + charge_ratio);
        } else {
                return 1.;
        }
}


/* Default reference flux model, in GeV^-1 m^-2 s^-1 sr^-1 */
static struct mulder_flux reference_flux(
    struct mulder_reference * reference,
    double height,
    double elevation,
    double kinetic_energy)
{
        struct mulder_flux result = {0.};
        if ((height >= reference->height_min) &&
            (height <= reference->height_max)) {
                const double deg = M_PI / 180.;
                const double cos_theta = cos((90. - elevation) * deg);
                result.value = flux_gccly(cos_theta, kinetic_energy);
                const double f = charge_fraction(MULDER_ANTIMUON);
                result.asymmetry = 2 * f - 1.;
        }
        return result;
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
static struct mulder_flux reference_table_flux(
    struct mulder_reference * reference,
    double height,
    double elevation,
    double kinetic_energy)
{
        struct reference_table * table = (void *)reference;
        struct mulder_flux result = {0.};

        /* Compute the interpolation indices and coefficients */
        const double dlk = log(table->k_max / table->k_min) /
            (table->n_k - 1);
        double hk = log(kinetic_energy / table->k_min) / dlk;
        if ((hk < 0.) || (hk > table->n_k - 1)) return result;
        const int ik = (int)hk;
        hk -= ik;

        const double deg = M_PI / 180;
        const double c = cos((90 - elevation) * deg);
        const double dc = (table->c_max - table->c_min) / (table->n_c - 1);
        double hc = (c - table->c_min) / dc;
        if ((hc < 0.) || (hc > table->n_c - 1)) return result;
        const int ic = (int)hc;
        hc -= ic;

        int ih;
        double hh;
        if (table->n_h > 1) {
                const double dh = (table->h_max - table->h_min) /
                    (table->n_h - 1);
                hh = (height - table->h_min) / dh;
                if ((hh < 0.) || (hh > table->n_h - 1)) return result;
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
        double flux[2] = {0.};
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
                        flux[i] = g0 * (1. - hh) + g1 * hh;
                else
                        flux[i] = exp(log(g0) * (1. - hh) + log(g1) * hh);
        }

        const double tmp = flux[0] + flux[1];
        if (tmp > 0.) {
                result.value = tmp;
                result.asymmetry = (flux[0] - flux[1]) / tmp;
        }
        return result;
}


/* Load a tabulated reference flux */
struct mulder_reference * mulder_reference_load_table(const char * path)
{
        FILE * fid = fopen(path, "rb");
        if (fid == NULL) {
                MULDER_ERROR("could not open %s", strlen(path), path);
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
        MULDER_ERROR("bad format (%s)", strlen(path), path);
        return NULL;
}


void mulder_reference_destroy_table(struct mulder_reference ** reference)
{
        if ((reference == NULL) || (*reference == NULL)) return;
        free(*reference);
        *reference = NULL;
}


/* Pumas PRNG wrapper */
static double random_pumas(struct pumas_context * context)
{
        struct fluxmeter * f = context->user_data;
        struct mulder_prng * prng = f->api.prng;
        return prng->uniform01(prng);
}


static unsigned long get_seed(struct mulder_prng * prng)
{
        struct fluxmeter * f = (void *)prng - offsetof(struct fluxmeter, prng);
        struct pumas_context * context = f->context;

        unsigned long seed;
        pumas_error_catch(1); /* Silently ignore unlikely error(s) below */
        pumas_context_random_seed_get(context, &seed);
        pumas_error_catch(0);
        return seed;
}


static void set_seed(
    struct mulder_prng * prng,
    const unsigned long * seed)
{
        struct fluxmeter * f = (void *)prng - offsetof(struct fluxmeter, prng);
        struct pumas_context * context = f->context;

        pumas_error_catch(1); /* Silently ignore unlikely error(s) below */
        pumas_context_random_seed_set(context, seed);
        pumas_error_catch(0);
}


static double uniform01(struct mulder_prng * prng)
{
        struct fluxmeter * f = (void *)prng - offsetof(struct fluxmeter, prng);
        struct pumas_context * context = f->context;

        return f->context_random(context);
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
