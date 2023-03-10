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
    int n,
    const double * position,
    double * height)
{
        for (; n > 0; n--, position+= 2, height++) {
                *height = mulder_layer_height(
                    layer,
                    *(struct mulder_projection *)position
                );
        }
}


/* Vectorized topography gradient */
void mulder_layer_gradient_v(
    const struct mulder_layer * layer,
    int n,
    const double * position,
    double * gradient)
{
        for (; n > 0; n--, position+= 2, gradient+= 2) {
                struct mulder_projection tmp = mulder_layer_gradient(
                    layer,
                    *(struct mulder_projection *)position
                );

                memcpy(
                    gradient,
                    &tmp,
                    sizeof tmp
                );
        }
}


/* Vectorized geographic coordinates */
void mulder_layer_coordinates_v(
    const struct mulder_layer * layer,
    int n,
    const double * map,
    double * geographic)
{
        for (; n > 0; n--, map+= 2, geographic+= 3) {
                struct mulder_coordinates tmp = mulder_layer_coordinates(
                    layer,
                    *(struct mulder_projection *)map
                );

                memcpy(
                    geographic,
                    &tmp,
                    sizeof tmp
                );
        }
}


/* Vectorized map projection */
void mulder_layer_project_v(
    const struct mulder_layer * layer,
    int n,
    const double * geographic,
    double * map)
{
        for (; n > 0; n--, geographic+= 3, map+= 2) {
                struct mulder_projection tmp = mulder_layer_project(
                    layer,
                    *(struct mulder_coordinates *)geographic
                );

                memcpy(
                    map,
                    &tmp,
                    sizeof tmp
                );
        }
}


/* Vectorized geomagnetif field */
void mulder_geomagnet_field_v(
    struct mulder_geomagnet * geomagnet,
    int n,
    const double * position,
    double * field)
{
        for (; n > 0; n--, position+= 3, field+= 3) {
                struct mulder_enu tmp = mulder_geomagnet_field(
                    geomagnet,
                    *(struct mulder_coordinates *)position
                );

                memcpy(
                    field,
                    &tmp,
                    sizeof(tmp)
                );
        }
}


/* Vectorized flux computation */
enum mulder_return mulder_fluxmeter_flux_v(
    struct mulder_fluxmeter * fluxmeter,
    const double * position,
    const double * direction,
    int n,
    const double * energy,
    double * flux)
{
        last_error.rc = MULDER_SUCCESS;
        for (; n > 0; n--, energy++, flux+= 2) {
                /* XXX update
                struct mulder_flux tmp = mulder_fluxmeter_flux(
                    fluxmeter,
                    *(struct mulder_coordinates *)position,
                    *(struct mulder_direction *)direction,
                    *energy
                );
                if (last_error.rc == MULDER_FAILURE) {
                        return MULDER_FAILURE;
                }

                memcpy(
                    flux,
                    &tmp,
                    sizeof tmp
                );
                */
        }
        return MULDER_SUCCESS;
}


/* Vectorized reference flux */
void mulder_reference_flux_v(
    struct mulder_reference * reference,
    double height,
    double elevation,
    int n,
    const double * energy,
    double * flux)
{
        for (; n > 0; n--, energy++, flux+= 2) {
                struct mulder_flux tmp = reference->flux(
                    reference,
                    height,
                    elevation,
                    *energy
                );

                memcpy(
                    flux,
                    &tmp,
                    sizeof tmp
                );
        }
}


/* Vectorized state flux */
void mulder_state_flux_v(
    struct mulder_reference * reference,
    int n,
    const int * pid,
    const double * data,
    double * flux)
{
        for (; n > 0; n--, pid++, data+= 7, flux+= 2) {
                struct mulder_state state = {.pid = *pid};
                memcpy(
                    &state.position,
                    data,
                    7 * sizeof(*data)
                );

                struct mulder_flux tmp = mulder_state_flux(
                    state,
                    reference
                );

                memcpy(
                    flux,
                    &tmp,
                    sizeof tmp
                );
        }
}


/* Vectorized transport */
enum mulder_return mulder_fluxmeter_transport_v(
    struct mulder_fluxmeter * fluxmeter,
    int n,
    const int * pid_in,
    const double * data_in,
    int * pid_out,
    double * data_out)
{
        last_error.rc = MULDER_SUCCESS;
        for (; n > 0; n--, pid_in++, data_in+= 7, pid_out++, data_out+= 7) {
                struct mulder_state tmp = {.pid = *pid_in};
                memcpy(
                    &tmp.position,
                    data_in,
                    7 * sizeof(*data_in)
                );

                tmp = mulder_fluxmeter_transport(fluxmeter, tmp);
                if (last_error.rc == MULDER_FAILURE) {
                        return MULDER_FAILURE;
                }

                *pid_out = tmp.pid;
                memcpy(
                    data_out,
                    &tmp.position,
                    7 * sizeof(*data_out)
                );
        }
        return MULDER_SUCCESS;
}


/* Vectorized intersections */
enum mulder_return mulder_fluxmeter_intersect_v(
    struct mulder_fluxmeter * fluxmeter,
    const double * position,
    int n,
    const double * direction,
    int * layer,
    double * intersection)
{
        last_error.rc = MULDER_SUCCESS;
        for (; n > 0; n--, direction+= 2, layer++, intersection+= 3) {
                struct mulder_intersection tmp = mulder_fluxmeter_intersect(
                    fluxmeter,
                    *(struct mulder_coordinates *)position,
                    *(struct mulder_direction *)direction
                );
                if (last_error.rc == MULDER_FAILURE) {
                        return MULDER_FAILURE;
                }

                *layer = tmp.layer;

                if ((tmp.layer < 0) || (tmp.layer >= fluxmeter->size)) {
                        memset(
                            intersection,
                            0x0,
                            sizeof tmp.position
                        );
                } else {
                        memcpy(
                            intersection,
                            &tmp.position,
                            sizeof tmp.position
                        );
                }
        }
        return MULDER_SUCCESS;
}


/* Vectorized grammage */
enum mulder_return mulder_fluxmeter_grammage_v(
    struct mulder_fluxmeter * fluxmeter,
    const double * position,
    int n,
    const double * direction,
    double * grammage)
{
        last_error.rc = MULDER_SUCCESS;
        const int m = fluxmeter->size + 1;
        for (; n > 0; n--, direction+= 2, grammage += m) {
                mulder_fluxmeter_grammage(
                    fluxmeter,
                    *(struct mulder_coordinates *)position,
                    *(struct mulder_direction *)direction,
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
    int n,
    const double * position,
    int * layer)
{
        last_error.rc = MULDER_SUCCESS;
        for (; n > 0; n--, position+= 3, layer++) {
                *layer = mulder_fluxmeter_whereami(
                    fluxmeter,
                    *(struct mulder_coordinates *)position
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
