/* This example illustrates usage of the mulder C library.
 *
 * This is a basic example showing how to setup an Earth geometry, from existing
 * data, and how to perform muon flux computations. For a more in depth
 * description of mulder, and for more sophisticated applications, you might
 * instead refer to the Python examples.
 *
 * Note that mulder C library is bare-bone in comparison to the Python package.
 * It has no built-in vectorisation nor high level functionalities, e.g. for
 * creating maps etc. Such operations are expected to be performed from Python.
 * Mulder C library instead provides a low level engine that can be integrated
 * with other C/C++ applications, e.g. as an atmospheric muons generator for a
 * detector Monte Carlo.
 *
 * Prerequisites:
 *
 * - This example uses the same topography data than the python `basic/layer.py`
 *   example. Please refer to the latter in order to obtain these (or similar)
 *   topography data.
 *
 * - This example also uses (geo)physical data bundled with the Python package.
 *   (see MULDER_PREFIX below).
 */

/* C standard library */
#include <stdlib.h>
#include <stdio.h>

/* Mulder API */
#include "mulder.h"


/* The MULDER_PREFIX macro, defined below, is assumed to point to the Python
 * package root directory. Depending on your use case, you have several options.
 *
 * - If this example is built from mulder source, using the provided Makefile,
 *   then `MULDER_PREFIX` should be properly overridden at compilation time.
 *   However, note that you also need to generate the corresponding data before
 *   running (e.g. by invoking `make package`).
 *
 * - If you compile this out of mulder source, then you might find useful the
 *   `mulder config --prefix` command, shipped with the mulder Python package.
 *
 * - Alternatively, you could simply edit MULDER_PREFIX below, and the related
 *   data files, in order to suite your needs. However, note that materials
 *   stopping power tables are required in any case, which can be generated with
 *   the Python package (e.g. as `mulder generate path/to/materials.xml`).
 */
#ifndef MULDER_PREFIX
#define MULDER_PREFIX "."
#endif


/* With this basic example, we perform all operations directly from `main'. In a
 * practical use case, one would instead separate the initialisation,
 * computation and finalisation steps. For example, in C++, a wrapper class
 * could take care of allocating resources in a constructor, and of releasing
 * them in the destructor.
 */
int main(int argc, char * argv[])
{
        /* Part I. Initialisation.
         *
         * In this first part we setup the scene. That is, a geometry is defined
         * using topography data. Geomagnetic data are also attached to the
         * geometry definition. From this definition, a fluxmeter is created.
         */

        /* To start with, we define a Stratified Earth geometry (SEG) using
         * two layers: a top layer made of Water, and a bottom layer of Rocks.
         * The rock surface is described by a Digital Elevation Model (DEM),
         * while the water layer has a constant height of zero.
         *
         * Note that in reading order the Water layer seems to be at the bottom
         * (which is not the case). Layers are actually ordered by indices. The
         * higher the index, the higher the layer. Thus, the Water layer of
         * index 1 is indeed the Rock layer of index 0.
         *
         * Please, see the `basic\geometry.py` and `basic\layer.py` Python
         * examples for more informations on layers and geometries.
         */
        struct mulder_layer * layers[] = {
            mulder_layer_create("Rock", "data/GMRT.asc", 0.),
            mulder_layer_create("Water", NULL, 0.)
        };
        const int n_layers = sizeof(layers) / sizeof(*layers);

        struct mulder_geometry * geometry = mulder_geometry_create(
            n_layers,
            layers
        );

        /* A geomagnetic field is attached to the previous geometry definition.
         * Note that this step is optional, and could be commented out. Note
         * also that, contrary to the SEG layout, the geomagnetic field can be
         * modified during the course of computations.
         */
        struct mulder_geomagnet * magnet = mulder_geomagnet_create(
            MULDER_PREFIX "/data/IGRF13.COF", /* Model */
            1,                                /* day */
            1,                                /* month */
            2020                              /* year */
        );
        geometry->geomagnet = magnet;

        /* A fluxmeter is created from the previous geometry definition.
         * Fluxmeters are the core object of mulder. They can be seen as local
         * probes of the atmospheric muons flux.
         */
        struct mulder_fluxmeter * fluxmeter = mulder_fluxmeter_create(
            MULDER_PREFIX "/data/materials.pumas", /* Materials tables */
            geometry
        );


        /* Part II. Flux computation.
         *
         * In this second part we compute the flux of atmospheric muons for some
         * observation state, using the previously created fluxmeter.
         */

        /* Mulder uses geographic coordinates in order to locate an observation
         * point. Below, we retrieve the position of the center of the map
         * describing the rock interface.
         */
        struct mulder_projection projection = {
            .x = 0.5 * (layers[0]->xmin + layers[0]->xmax),
            .y = 0.5 * (layers[0]->ymin + layers[0]->ymax)
        };

        struct mulder_position position = mulder_layer_position(
            layers[0],
            projection
        );

        /* The height of the previous position corresponds to the rock interface
         * with the atmosphere. Let us move this position 30m below the ground.
         */
        position.height -= 30; /* m */

        /* Using the previous position, we now define a complete observation
         * state.
         */
        struct mulder_state state = {
            .position = position,
            .direction = {
                .azimuth = 90.,   /* deg, clockwise w.r.t. North. */
                .elevation = 30.  /* deg, w.r.t. the local horizontal. */
            },
            .energy = 1E+01,      /* GeV */
            .weight = 1
        };

        /* Then, the corresponding flux is simply obtained as below. */
        struct mulder_flux flux = mulder_fluxmeter_flux(fluxmeter, state);

        /* Let us print the result. */
        printf("flux = %.5E GeV^-1 m^-2 s^-1 sr^1 (%+.5f)\n",
            flux.value, flux.asymmetry);

        /* Part III. Finalisation.
         *
         * In this last step, we destroy all mulde objects, releasing the
         * corresponding dynamically allocated memory. After this stage,
         * valgrind should be happy.
         */
        int i;
        for (i = 0; i < n_layers; i++) {
                mulder_layer_destroy(layers + i);
        }
        mulder_geometry_destroy(&geometry);
        mulder_geomagnet_destroy(&magnet);
        mulder_fluxmeter_destroy(&fluxmeter);

        exit(EXIT_SUCCESS);
}
