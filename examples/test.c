/* C standard library */
#include <stdlib.h>
#include <stdio.h>

/* Mulder API */
#include "mulder.h"


int main(int argc, char * argv[])
{
        /* Define a stratified Earth geometry */
        struct mulder_layer * layers[] = {
            mulder_layer_create("Rock", "data/mns_roche.png", 0.),
            mulder_layer_create("Water", "data/mns_eau.png", 0.)
        };
        const int n_layers = sizeof(layers) / sizeof(*layers);

        struct mulder_geometry * geometry = mulder_geometry_create(
            n_layers,
            layers
        );

        /* Attach a geomagnetic field (optional) */
        struct mulder_geomagnet * magnet = mulder_geomagnet_create(
            "mulder/data/IGRF13.COF",
            1,   /* day */
            1,   /* month */
            2020 /* year */
        );
        geometry->geomagnet = magnet;

        /* Create the fluxmeter */
        struct mulder_fluxmeter * fluxmeter = mulder_fluxmeter_create(
            "mulder/data/materials.pumas",
            geometry
        );

        /* Get geographic position at the middle of the map, and offset the
         * height below the ground
         */
        struct mulder_projection projection = {
            .x = 0.5 * (layers[0]->xmin + layers[0]->xmax),
            .y = 0.5 * (layers[0]->ymin + layers[0]->ymax)
        };

        struct mulder_position position = mulder_layer_position(
            layers[0],
            projection
        );
        position.height -= 30;

        /* Define an observation state and compute the corresponding flux */
        struct mulder_state state = {
            .position = position,
            .direction = {
                .azimuth = 0.,
                .elevation = 90.
            },
            .energy = 1E+01
        };
        struct mulder_flux flux = mulder_fluxmeter_flux(fluxmeter, state);

        printf("flux = %.5E GeV^-1 m^-2 s^-1 sr^1 (%+.5f)\n",
            flux.value, flux.asymmetry);

        /* Free memory */
        int i;
        for (i = 0; i < n_layers; i++) {
                mulder_layer_destroy(layers + i);
        }
        mulder_geometry_destroy(&geometry);
        mulder_geomagnet_destroy(&magnet);
        mulder_fluxmeter_destroy(&fluxmeter);

        exit(EXIT_SUCCESS);
}
