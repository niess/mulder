/* C standard library */
#include <stdlib.h>
#include <stdio.h>

/* Mulder API */
#include "mulder.h"


int main(int argc, char * argv[])
{
        /* Define the geometry */
        struct mulder_layer * layers[] = {
            mulder_layer_create("Rock", "data/mns_roche.png", 0.),
            mulder_layer_create("Water", "data/mns_eau.png", 0.)
        };
        const int n_layers = sizeof(layers) / sizeof(*layers);

        /* Create the fluxmeter */
        struct mulder_fluxmeter * meter =
            mulder_fluxmeter_create("mulder/data/materials.pumas",
                n_layers, layers);

        /* Attach a geomagnetic field (optional) */
        struct mulder_geomagnet * magnet =
            mulder_geomagnet_create("mulder/data/IGRF13.COF", 1, 1, 2020);
        meter->geomagnet = magnet;

        /* Get geographic coordinates at the middle of the map, and offset
         * the height below the ground
         */
        const double x = 0.5 * (layers[0]->xmin + layers[0]->xmax);
        const double y = 0.5 * (layers[0]->ymin + layers[0]->ymax);

        struct mulder_coordinates position =
            mulder_layer_coordinates(layers[0], x, y);
        position.height -= 30;

        /* Compute the muon flux along some observation direction */
        struct mulder_direction direction = {.azimuth = 0., .elevation = 90.};
        const double kinetic_energy = 1E+01;
        struct mulder_flux flux = mulder_fluxmeter_flux(
            meter, position, direction, kinetic_energy);

        printf("flux = %.5E GeV^-1 m^-2 s^-1 sr^1 (%+.5f)\n",
            flux.value, flux.asymmetry);

        /* Free memory */
        int i;
        for (i = 0; i < n_layers; i++) {
                mulder_layer_destroy(layers + i);
        }
        mulder_geomagnet_destroy(&magnet);
        mulder_fluxmeter_destroy(&meter);

        exit(EXIT_SUCCESS);
}
