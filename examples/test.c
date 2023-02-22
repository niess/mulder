/* C standard library */
#include <stdlib.h>
#include <stdio.h>

/* Mulder API */
#include "mulder.h"


int main(int argc, char * argv[])
{
        /* Define the geometry */
        struct mulder_layer * layers[] = {
            mulder_layer_create("StandardRock", "data/mns_roche.png", 0.),
            mulder_layer_create("Water", "data/mns_eau.png", 0.)
        };
        const int n_layers = sizeof(layers) / sizeof(*layers);

        /* Create the fluxmeter */
        struct mulder_fluxmeter * meter =
            mulder_fluxmeter_create("deps/pumas/examples/data/materials.pumas",
                n_layers, layers);

        /* Get geodetic coordinates at the middle of the map */
        const double x = 0.5 * (layers[0]->xmin + layers[0]->xmax);
        const double y = 0.5 * (layers[0]->ymin + layers[0]->ymax);

        double latitude, longitude, height;
        height = mulder_layer_height(layers[0], x, y);
        mulder_layer_geodetic(layers[0], x, y, &latitude, &longitude);

        /* Compute the muon flux along some observation direction */
        const double kinetic_energy = 1E+01;
        const double azimuth = 0.;
        const double elevation = 90.;
        const double flux = mulder_fluxmeter_flux(meter, kinetic_energy,
            latitude, longitude, height - 30., azimuth, elevation);

        printf("flux = %12.5E GeV^-1 m^-2 s^-1 sr^1\n", flux);

        /* Free memory */
        int i;
        for (i = 0; i < n_layers; i++) {
                mulder_layer_destroy(&layers[i]);
        }
        mulder_fluxmeter_destroy(&meter);

        exit(EXIT_SUCCESS);
}
