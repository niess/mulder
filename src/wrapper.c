/* C standard library */
#include <float.h>

/* Custom APIs */
#include "mulder.h"
#include "turtle.h"


/* XXX catch errors */


/* Vectorized topography height */
void mulder_layer_height_v(
    const struct mulder_layer * layer, int nx, int ny, const double * x,
    const double * y, double * z)
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
    const struct mulder_layer * layer, int n, const double * x,
    const double * y, double * gx, double * gy)
{
        for (; n > 0; n--, x++, y++, gx++, gy++) {
                mulder_layer_gradient(layer, *x, *y, gx, gy);
        }
}


/* Vectorized geodetic coordinates */
void mulder_layer_geodetic_v(
    const struct mulder_layer * layer, int n, const double * x,
    const double * y, double * latitude, double * longitude)
{
        for (; n > 0; n--, x++, y++, latitude++, longitude++) {
                mulder_layer_geodetic(layer, *x, *y, latitude, longitude);
        }
}


/* Vectorized map coordinates */
void mulder_layer_coordinates_v(
    const struct mulder_layer * layer, int n, const double * latitude,
    const double * longitude, double * x, double * y)
{
        for (; n > 0; n--, latitude++, longitude++, x++, y++) {
                mulder_layer_coordinates(layer, *latitude, *longitude, x, y);
        }
}


/* Vectorized flux computation */
void mulder_fluxmeter_flux_v(
    struct mulder_fluxmeter * fluxmeter, double latitude, double longitude,
    double height, double azimuth, double elevation, int n,
    const double * energy, double * flux)
{
        for (; n > 0; n--, energy++, flux++) {
                *flux = mulder_fluxmeter_flux(fluxmeter, *energy, latitude,
                    longitude, height, azimuth, elevation);
        }
}


/* Vectorized reference flux */
void mulder_fluxmeter_reference_flux_v(
    struct mulder_fluxmeter * fluxmeter, double elevation, int n,
    const double * energy, double * flux)
{
        for (; n > 0; n--, energy++, flux++) {
                *flux = fluxmeter->reference_flux(*energy, elevation);
        }
}


/* Vectorized intersections */
void mulder_fluxmeter_intersect_v(
    struct mulder_fluxmeter * fluxmeter, double latitude, double longitude,
    double height, int n, const double * azimuth, const double * elevation,
    int * layer, double * x, double * y, double * z)
{
        for (; n > 0; n--, azimuth++, elevation++, layer++, x++, y++, z++) {
                double la = latitude, lo = longitude;
                *z = height;
                *layer = mulder_fluxmeter_intersect(
                    fluxmeter, &la, &lo, z, *azimuth, *elevation);
                if ((*layer < 0) || (*layer >= fluxmeter->size)) {
                        *x = *y = *z = 0.;
                } else {
                        mulder_layer_coordinates(
                            fluxmeter->layers[*layer], la, lo, x, y);
                }
        }
}


/* Vectorized grammage */
void mulder_fluxmeter_grammage_v(
    struct mulder_fluxmeter * fluxmeter, double latitude, double longitude,
    double height, int n, const double * azimuth, const double * elevation,
    double * grammage)
{
        const int m = fluxmeter->size + 1;
        for (; n > 0; n--, azimuth++, elevation++, grammage += m) {
                mulder_fluxmeter_grammage(fluxmeter, latitude, longitude,
                    height, *azimuth, *elevation, grammage);
        }
}


/* Vectorized locator */
void mulder_fluxmeter_whereami_v(
    struct mulder_fluxmeter * fluxmeter, int n, const double * latitude,
    const double * longitude, const double * height, int * layer)
{
        for (; n > 0; n--, latitude++, longitude++, height++, layer++) {
                *layer = mulder_fluxmeter_whereami(
                    fluxmeter, *latitude, *longitude, *height);
        }
}


/* Create a TURTLE map from raw data */
void mulder_map_create(const char * path, const char * projection,
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

        struct turtle_map * map;
        turtle_map_create(&map, &info, projection);

        for (i = 0, zi = z; i < ny; i++) {
                int j;
                for (j = 0; j < nx; j++, zi++) {
                        turtle_map_fill(map, j, i, *zi);
                }
        }

        turtle_map_dump(map, path);
        turtle_map_destroy(&map);
}
