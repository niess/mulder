/* Error handling */
const char * mulder_error_get(void);
void mulder_error_clear(void);

/* Return codes */
enum mulder_return {
    MULDER_SUCCESS = 0,
    MULDER_FAILURE
};

/* Vectorized layer height */
void mulder_layer_height_v(
    const struct mulder_layer * layer,
    int size,
    const struct mulder_projection * projection,
    double * height
);

/* Vectorized layer gradient */
void mulder_layer_gradient_v(
    const struct mulder_layer * layer,
    int size,
    const struct mulder_projection * projection,
    struct mulder_projection * gradient
);

/* Vectorized geographic coordinates */
void mulder_layer_coordinates_v(
    const struct mulder_layer * layer,
    int size,
    const struct mulder_projection * projection,
    struct mulder_coordinates * position
);

/* Vectorized map projection */
void mulder_layer_project_v(
    const struct mulder_layer * layer,
    int size,
    const struct mulder_coordinates * position,
    struct mulder_projection * projection
);

/* Vectorized geomagnetic field */
void mulder_geomagnet_field_v(
    struct mulder_geomagnet * geomagnet,
    int size,
    const struct mulder_coordinates * position,
    struct mulder_enu * field
);

/* Vectorized flux computation */
enum mulder_return mulder_fluxmeter_flux_v(
    struct mulder_fluxmeter * fluxmeter,
    int size,
    const struct mulder_state * state,
    struct mulder_flux * flux
);

/* Vectorized reference flux */
void mulder_reference_flux_v(
    struct mulder_reference * reference,
    int size,
    const double * height,
    const double * elevation,
    const double * energy,
    struct mulder_flux * flux
);

/* Vectorized state flux */
void mulder_state_flux_v(
    struct mulder_reference * reference,
    int size,
    const struct mulder_state * state,
    struct mulder_flux * flux
);

/* Vectorized transport */
enum mulder_return mulder_fluxmeter_transport_v(
    struct mulder_fluxmeter * fluxmeter,
    int size,
    const struct mulder_state * in,
    struct mulder_state * out
);

/* Vectorized intersections */
enum mulder_return mulder_fluxmeter_intersect_v(
    struct mulder_fluxmeter * fluxmeter,
    int size,
    const struct mulder_coordinates * position,
    const struct mulder_direction * direction,
    struct mulder_intersection * intersection
);

/* Vectorized gramage */
enum mulder_return mulder_fluxmeter_grammage_v(
    struct mulder_fluxmeter * fluxmeter,
    int size,
    const struct mulder_coordinates * position,
    const struct mulder_direction * direction,
    double * grammage
);

/* Vectorized locator */
enum mulder_return mulder_fluxmeter_whereami_v(
    struct mulder_fluxmeter * fluxmeter,
    int size,
    const struct mulder_coordinates * position,
    int * layer
);

/* Vectorized pseudo-random numbers */
void mulder_prng_uniform01_v(
    struct mulder_prng * prng,
    int n,
    double * values
);

/* Create a Turtle map from raw data */
enum mulder_return mulder_map_create(
    const char * path,
    const char * projection,
    int nx,
    int ny,
    double xmin,
    double xmax,
    double ymin,
    double ymax,
    const double * z
);

/* Generate physics tables for Pumas */
enum mulder_return mulder_generate_physics(
    const char * path,
    const char * destination,
    const char * dump
);
