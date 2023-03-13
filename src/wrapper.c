/* C standard library */
#include <float.h>
#include <stdlib.h>
#include <string.h>

/* Custom APIs */
#include "mulder.h"
#include "pumas.h"
#include "turtle.h"
#include "wrapper.h"


/* Data relative to the last captured error */
static struct {
        enum mulder_return rc;
        int size;
        char * msg;
} last_error = {MULDER_SUCCESS, 0, NULL};


/* Capture error messages */
static void capture_error(const char * message)
{
        last_error.rc = MULDER_FAILURE;
        const int n = strlen(message) + 1;
        if (n > last_error.size) {
                last_error.msg = realloc(last_error.msg, n);
                last_error.size = n;
        }
        memcpy(last_error.msg, message, n);
}


static void (*default_error)(const char * msg) = NULL;


__attribute__((constructor))
static void initialise_wrapper(void)
{
        default_error = mulder_error;
        mulder_error = &capture_error;
}


__attribute__((destructor))
static void finalise_wrapper(void)
{
        mulder_error = default_error;
        default_error = NULL;
        mulder_error_clear();
}


/* Error handling API */
const char * mulder_error_get(void)
{
        return last_error.msg;
}


void mulder_error_clear(void)
{
        free(last_error.msg);
        last_error.rc = MULDER_SUCCESS;
        last_error.msg = NULL;
        last_error.size = 0;
}


/* Vectorized topography height */
void mulder_layer_height_v(
    const struct mulder_layer * layer,
    int size,
    const struct mulder_projection * projection,
    double * height)
{
        for (; size > 0; size--, projection++, height++) {
                *height = mulder_layer_height(
                    layer,
                    *projection
                );
        }
}


/* Vectorized topography gradient */
void mulder_layer_gradient_v(
    const struct mulder_layer * layer,
    int size,
    const struct mulder_projection * projection,
    struct mulder_projection * gradient)
{
        for (; size > 0; size--, projection++, gradient++) {
                *gradient = mulder_layer_gradient(
                    layer,
                    *projection
                );
        }
}


/* Vectorized geographic position */
void mulder_layer_position_v(
    const struct mulder_layer * layer,
    int size,
    const struct mulder_projection * projection,
    struct mulder_position * position)
{
        for (; size > 0; size--, projection++, position++) {
                *position = mulder_layer_position(
                    layer,
                    *projection
                );
        }
}


/* Vectorized map projection */
void mulder_layer_project_v(
    const struct mulder_layer * layer,
    int size,
    const struct mulder_position * position,
    struct mulder_projection * projection)
{
        for (; size > 0; size--, position++, projection++) {
                *projection = mulder_layer_project(
                    layer,
                    *position
                );
        }
}


/* Vectorized geomagnetic field */
void mulder_geomagnet_field_v(
    struct mulder_geomagnet * geomagnet,
    int size,
    const struct mulder_position * position,
    struct mulder_enu * field)
{
        for (; size > 0; size--, position++, field++) {
                *field = mulder_geomagnet_field(
                    geomagnet,
                    *position
                );
        }
}


/* Vectorized flux computation */
enum mulder_return mulder_fluxmeter_flux_v(
    struct mulder_fluxmeter * fluxmeter,
    int size,
    const struct mulder_state * state,
    struct mulder_flux * flux)
{
        last_error.rc = MULDER_SUCCESS;
        for (; size > 0; size--, state++, flux++) {
                *flux = mulder_fluxmeter_flux(
                    fluxmeter,
                    *state
                );
                if (last_error.rc == MULDER_FAILURE) {
                        return MULDER_FAILURE;
                }
        }
        return MULDER_SUCCESS;
}


/* Vectorized reference flux */
void mulder_reference_flux_v(
    struct mulder_reference * reference,
    int size,
    const double * height,
    const double * elevation,
    const double * energy,
    struct mulder_flux * flux)
{
        for (; size > 0; size--, height++, elevation++, energy++, flux++) {
                *flux = reference->flux(
                    reference,
                    *height,
                    *elevation,
                    *energy
                );
        }
}


/* Vectorized state flux */
void mulder_state_flux_v(
    struct mulder_reference * reference,
    int size,
    const struct mulder_state * state,
    struct mulder_flux * flux)
{
        for (; size > 0; size--, state++, flux++) {
                *flux = mulder_state_flux(
                    *state,
                    reference
                );
        }
}


/* Vectorized transport */
enum mulder_return mulder_fluxmeter_transport_v(
    struct mulder_fluxmeter * fluxmeter,
    int size,
    const struct mulder_state * in,
    struct mulder_state * out)
{
        last_error.rc = MULDER_SUCCESS;
        for (; size > 0; size--, in++, out++) {
                *out = mulder_fluxmeter_transport(
                    fluxmeter,
                    *in
                );
                if (last_error.rc == MULDER_FAILURE) {
                        return MULDER_FAILURE;
                }
        }
        return MULDER_SUCCESS;
}


/* Vectorized intersections */
enum mulder_return mulder_fluxmeter_intersect_v(
    struct mulder_fluxmeter * fluxmeter,
    int size,
    const struct mulder_position * position,
    const struct mulder_direction * direction,
    struct mulder_intersection * intersection)
{
        last_error.rc = MULDER_SUCCESS;
        for (; size > 0; size--, position++, direction++, intersection++) {
                *intersection = mulder_fluxmeter_intersect(
                    fluxmeter,
                    *position,
                    *direction
                );
                if (last_error.rc == MULDER_FAILURE) {
                        return MULDER_FAILURE;
                }
        }
        return MULDER_SUCCESS;
}


/* Vectorized grammage */
enum mulder_return mulder_fluxmeter_grammage_v(
    struct mulder_fluxmeter * fluxmeter,
    int size,
    const struct mulder_position * position,
    const struct mulder_direction * direction,
    double * grammage)
{
        last_error.rc = MULDER_SUCCESS;
        const int m = fluxmeter->size + 1;
        for (; size > 0; size--, position++, direction++, grammage+= m) {
                mulder_fluxmeter_grammage(
                    fluxmeter,
                    *position,
                    *direction,
                    grammage
                );
                if (last_error.rc == MULDER_FAILURE) {
                        return MULDER_FAILURE;
                }
        }
        return MULDER_SUCCESS;
}


/* Vectorized locator */
enum mulder_return mulder_fluxmeter_whereami_v(
    struct mulder_fluxmeter * fluxmeter,
    int size,
    const struct mulder_position * position,
    int * layer)
{
        last_error.rc = MULDER_SUCCESS;
        for (; size > 0; size--, position++, layer++) {
                *layer = mulder_fluxmeter_whereami(
                    fluxmeter,
                    *position
                );
                if (last_error.rc == MULDER_FAILURE) {
                        return MULDER_FAILURE;
                }
        }
        return MULDER_SUCCESS;
}


/* Vectorized pseudo-random numbers */
void mulder_prng_uniform01_v(
    struct mulder_prng * prng,
    int n,
    double * values)
{
        for (; n > 0; n--, values++) {
                *values = prng->uniform01(prng);
        }
}


/* Create a TURTLE map from raw data */
enum mulder_return mulder_map_create(
    const char * path,
    const char * projection,
    int nx, int ny,
    double xmin,
    double xmax,
    double ymin,
    double ymax,
    const double * z)
{
        struct turtle_map_info info = {
                .nx = nx,
                .ny = ny,
                .x = {xmin, xmax},
                .y = {ymin, ymax}
        };

        const int n = nx * ny;
        int i;
        double zmin = DBL_MAX, zmax = -DBL_MIN;
        const double * zi;
        for (i = 0, zi = z; i < n; i++, zi++) {
                if (*zi < zmin) zmin = *zi;
                if (*zi > zmax) zmax = *zi;
        }
        info.z[0] = zmin;
        info.z[1] = zmax;

        last_error.rc = MULDER_SUCCESS;
        struct turtle_map * map;
        turtle_map_create(&map, &info, projection);
        if (last_error.rc == MULDER_FAILURE) {
                return MULDER_FAILURE;
        }

        for (i = 0, zi = z; i < ny; i++) {
                int j;
                for (j = 0; j < nx; j++, zi++) {
                        turtle_map_fill(map, j, i, *zi);
                }
        }

        turtle_map_dump(map, path);
        turtle_map_destroy(&map);

        return last_error.rc;
}


/* Generate physics tables for Pumas */
enum mulder_return mulder_generate_physics(
    const char * path,
    const char * destination,
    const char * dump)
{
        /* Pre-compute physics data */
        struct pumas_physics * physics;
        {
                enum pumas_return tmp = pumas_physics_create(
                    &physics,
                    PUMAS_PARTICLE_MUON,
                    path,
                    destination,
                    NULL
                );
                if (tmp != PUMAS_RETURN_SUCCESS) {
                        return MULDER_FAILURE;
                }
        }

        /* Dump the result */
        enum mulder_return rc = MULDER_FAILURE;
        FILE * stream = fopen(dump, "wb");
        if (stream != NULL) {
                enum pumas_return tmp = pumas_physics_dump(physics, stream);
                if (tmp == PUMAS_RETURN_SUCCESS) {
                        rc = MULDER_SUCCESS;
                }
                fclose(stream);
        } else {
                const char format[] = "could not open %s";
                const int n = sizeof(format) + strlen(dump);
                char msg[n];
                sprintf(msg, dump);
                mulder_error(msg);
        }
        pumas_physics_destroy(&physics);

        return rc;
}
