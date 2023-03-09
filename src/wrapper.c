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
    int nx,
    int ny,
    const double * x,
    const double * y,
    double * z)
{
        for (; ny > 0; ny--, y++) {
                int i;
                const double * xi;
                for (i = 0, xi = x; i < nx; i++, xi++, z++) {
                        *z = mulder_layer_height(layer, *xi, *y);
                }
        }
}


/* Vectorized topography gradient */
void mulder_layer_gradient_v(
    const struct mulder_layer * layer,
    int n,
    const double * x,
    const double * y,
    double * gx,
    double * gy)
{
        for (; n > 0; n--, x++, y++, gx++, gy++) {
                mulder_layer_gradient(layer, *x, *y, gx, gy);
        }
}


/* Vectorized geographic coordinates */
void mulder_layer_coordinates_v(
    const struct mulder_layer * layer,
    int n,
    const double * x,
    const double * y,
    double * coordinates)
{
        for (; n > 0; n--, x++, y++, coordinates+= 3) {
                struct mulder_coordinates tmp =
                    mulder_layer_coordinates(layer, *x, *y);
                memcpy(coordinates, &tmp, sizeof(tmp));
        }
}


/* Vectorized map projection */
void mulder_layer_project_v(
    const struct mulder_layer * layer,
    int n,
    const double * coordinates,
    double * x,
    double * y)
{
        for (; n > 0; n--, coordinates+= 3, x++, y++) {
                mulder_layer_project(layer, (const void *)coordinates, x, y);
        }
}


/* Vectorized geomagnetif field */
void mulder_geomagnet_field_v(
    struct mulder_geomagnet * geomagnet,
    int n,
    const double * coordinates,
    double * field)
{
        for (; n > 0; n--, coordinates+= 3, field += 3) {
                struct mulder_enu enu = mulder_geomagnet_field(
                    geomagnet, (const void *)coordinates);
                memcpy(field, &enu, sizeof(enu));
        }
}


/* Vectorized flux computation */
enum mulder_return mulder_fluxmeter_flux_v(
    struct mulder_fluxmeter * fluxmeter, double latitude, double longitude,
    double height, double azimuth, double elevation, int n,
    const double * energy, double * result)
{
        last_error.rc = MULDER_SUCCESS;
        for (; n > 0; n--, energy++, result+= 2) {
                struct mulder_flux flux = mulder_fluxmeter_flux(fluxmeter,
                    *energy, latitude, longitude, height, azimuth, elevation);
                if (last_error.rc == MULDER_FAILURE) {
                        return MULDER_FAILURE;
                }
                result[0] = flux.value;
                result[1] = flux.asymmetry;
        }
        return MULDER_SUCCESS;
}


/* Vectorized reference flux */
void mulder_reference_flux_v(struct mulder_reference * reference,
    double height, double elevation, int n, const double * energy,
    double * result)
{
        for (; n > 0; n--, energy++, result+= 2) {
                struct mulder_flux flux = reference->flux(
                    reference, height, elevation, *energy);
                result[0] = flux.value;
                result[1] = flux.asymmetry;
        }
}


/* Vectorized state flux */
void mulder_state_flux_v(struct mulder_reference * reference,
    int n, const int * pid, const double * data, double * result)
{
        for (; n > 0; n--, pid++, data+= 7, result+= 2) {
                /* XXX use memcpy? */
                struct mulder_state state = {
                        .pid = *pid,
                        .latitude = data[0],
                        .longitude = data[1],
                        .height = data[2],
                        .azimuth = data[3],
                        .elevation = data[4],
                        .energy = data[5],
                        .weight = data[6]
                };
                struct mulder_flux flux = mulder_state_flux(
                    &state, reference);
                result[0] = flux.value;
                result[1] = flux.asymmetry;
        }
}


/* Vectorized transport */
enum mulder_return mulder_fluxmeter_transport_v(
    struct mulder_fluxmeter * fluxmeter, int n, const int * pid_in,
    const double * data_in, int * pid_out, double * data_out)
{
        last_error.rc = MULDER_SUCCESS;
        for (; n > 0; n--, pid_in++, data_in+= 7, pid_out++, data_out+= 7) {
                struct mulder_state s = {
                        .pid = *pid_in,
                        .latitude = data_in[0],
                        .longitude = data_in[1],
                        .height = data_in[2],
                        .azimuth = data_in[3],
                        .elevation = data_in[4],
                        .energy = data_in[5],
                        .weight = data_in[6]
                };
                s = mulder_fluxmeter_transport(fluxmeter, &s);
                if (last_error.rc == MULDER_FAILURE) {
                        return MULDER_FAILURE;
                }
                *pid_out = s.pid;
                data_out[0] = s.latitude;
                data_out[1] = s.longitude;
                data_out[2] = s.height;
                data_out[3] = s.azimuth;
                data_out[4] = s.elevation;
                data_out[5] = s.energy;
                data_out[6] = s.weight;
        }
        return MULDER_SUCCESS;
}


/* Vectorized intersections */
enum mulder_return mulder_fluxmeter_intersect_v(
    struct mulder_fluxmeter * fluxmeter, double latitude, double longitude,
    double height, int n, const double * azimuth, const double * elevation,
    int * layer, double * x, double * y, double * z)
{
        last_error.rc = MULDER_SUCCESS;
        for (; n > 0; n--, azimuth++, elevation++, layer++, x++, y++, z++) {
                double la = latitude, lo = longitude;
                *z = height;
                *layer = mulder_fluxmeter_intersect(
                    fluxmeter, &la, &lo, z, *azimuth, *elevation);
                if (last_error.rc == MULDER_FAILURE) {
                        return MULDER_FAILURE;
                }
                if ((*layer < 0) || (*layer >= fluxmeter->size)) {
                        *x = *y = *z = 0.;
                } else {
                        struct mulder_coordinates tmp = {la, lo, 0.};
                        mulder_layer_project(
                            fluxmeter->layers[*layer], &tmp, x, y);
                }
        }
        return MULDER_SUCCESS;
}


/* Vectorized grammage */
enum mulder_return mulder_fluxmeter_grammage_v(
    struct mulder_fluxmeter * fluxmeter, double latitude, double longitude,
    double height, int n, const double * azimuth, const double * elevation,
    double * grammage)
{
        last_error.rc = MULDER_SUCCESS;
        const int m = fluxmeter->size + 1;
        for (; n > 0; n--, azimuth++, elevation++, grammage += m) {
                mulder_fluxmeter_grammage(fluxmeter, latitude, longitude,
                    height, *azimuth, *elevation, grammage);
                if (last_error.rc == MULDER_FAILURE) {
                        return MULDER_FAILURE;
                }
        }
        return MULDER_SUCCESS;
}


/* Vectorized locator */
enum mulder_return mulder_fluxmeter_whereami_v(
    struct mulder_fluxmeter * fluxmeter, int n, const double * latitude,
    const double * longitude, const double * height, int * layer)
{
        last_error.rc = MULDER_SUCCESS;
        for (; n > 0; n--, latitude++, longitude++, height++, layer++) {
                *layer = mulder_fluxmeter_whereami(
                    fluxmeter, *latitude, *longitude, *height);
                if (last_error.rc == MULDER_FAILURE) {
                        return MULDER_FAILURE;
                }
        }
        return MULDER_SUCCESS;
}


/* Vectorized pseudo-random numbers */
void mulder_prng_uniform01_v(struct mulder_prng * prng, int n, double * values)
{
        for (; n > 0; n--, values++) {
                *values = prng->uniform01(prng);
        }
}


/* Create a TURTLE map from raw data */
enum mulder_return mulder_map_create(const char * path, const char * projection,
    int nx, int ny, double xmin, double xmax, double ymin, double ymax,
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
    const char * path, const char * destination, const char * dump)
{
        /* Pre-compute physics data */
        struct pumas_physics * physics;
        if (pumas_physics_create(
            &physics, PUMAS_PARTICLE_MUON, path, destination, NULL) !=
            PUMAS_RETURN_SUCCESS) {
                return MULDER_FAILURE;
        }

        /* Dump the result */
        enum mulder_return rc = MULDER_FAILURE;
        FILE * stream = fopen(dump, "wb");
        if (stream != NULL) {
                if (pumas_physics_dump(physics, stream) ==
                    PUMAS_RETURN_SUCCESS) {
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
