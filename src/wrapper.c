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
enum mulder_return mulder_layer_height_v(
    const struct mulder_layer * layer, int nx, int ny, const double * x,
    const double * y, double * z)
{
        last_error.rc = MULDER_SUCCESS;
        for (; ny > 0; ny--, y++) {
                int i;
                const double * xi;
                for (i = 0, xi = x; i < nx; i++, xi++, z++) {
                        *z = mulder_layer_height(layer, *xi, *y);
                        if (last_error.rc == MULDER_FAILURE) {
                                return MULDER_FAILURE;
                        }
                }
        }
        return MULDER_SUCCESS;
}


/* Vectorized topography gradient */
enum mulder_return mulder_layer_gradient_v(
    const struct mulder_layer * layer, int n, const double * x,
    const double * y, double * gx, double * gy)
{
        last_error.rc = MULDER_SUCCESS;
        for (; n > 0; n--, x++, y++, gx++, gy++) {
                mulder_layer_gradient(layer, *x, *y, gx, gy);
                if (last_error.rc == MULDER_FAILURE) {
                        return MULDER_FAILURE;
                }
        }
        return MULDER_SUCCESS;
}


/* Vectorized geodetic coordinates */
enum mulder_return mulder_layer_geodetic_v(
    const struct mulder_layer * layer, int n, const double * x,
    const double * y, double * latitude, double * longitude)
{
        last_error.rc = MULDER_SUCCESS;
        for (; n > 0; n--, x++, y++, latitude++, longitude++) {
                mulder_layer_geodetic(layer, *x, *y, latitude, longitude);
                if (last_error.rc == MULDER_FAILURE) {
                        return MULDER_FAILURE;
                }
        }
        return MULDER_SUCCESS;
}


/* Vectorized map coordinates */
enum mulder_return mulder_layer_coordinates_v(
    const struct mulder_layer * layer, int n, const double * latitude,
    const double * longitude, double * x, double * y)
{
        last_error.rc = MULDER_SUCCESS;
        for (; n > 0; n--, latitude++, longitude++, x++, y++) {
                mulder_layer_coordinates(layer, *latitude, *longitude, x, y);
                if (last_error.rc == MULDER_FAILURE) {
                        return MULDER_FAILURE;
                }
        }
        return MULDER_SUCCESS;
}


/* Vectorized flux computation */
enum mulder_return mulder_fluxmeter_flux_v(
    struct mulder_fluxmeter * fluxmeter, double latitude, double longitude,
    double height, double azimuth, double elevation, int n,
    const double * energy, double * flux)
{
        last_error.rc = MULDER_SUCCESS;
        for (; n > 0; n--, energy++, flux++) {
                *flux = mulder_fluxmeter_flux(fluxmeter, *energy, latitude,
                    longitude, height, azimuth, elevation);
                if (last_error.rc == MULDER_FAILURE) {
                        return MULDER_FAILURE;
                }
        }
        return MULDER_SUCCESS;
}


/* Vectorized reference flux */
enum mulder_return mulder_reference_flux_v(
    struct mulder_reference * reference, double height, double elevation, int n,
    const double * energy, double * flux)
{
        last_error.rc = MULDER_SUCCESS;
        for (; n > 0; n--, energy++, flux++) {
                *flux = reference->flux(reference, height, elevation, *energy);
                if (last_error.rc == MULDER_FAILURE) {
                        return MULDER_FAILURE;
                }
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
                        mulder_layer_coordinates(
                            fluxmeter->layers[*layer], la, lo, x, y);
                }
                if (last_error.rc == MULDER_FAILURE) {
                        return MULDER_FAILURE;
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
