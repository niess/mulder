/* Vectorized layer height */
void mulder_layer_height_v(
    const struct mulder_layer * layer,
    int nx,
    int ny,
    const double * x,
    const double * y,
    double * z);

/* Vectorized layer gradient */
void mulder_layer_gradient_v(
    const struct mulder_layer * layer,
    int n,
    const double * x,
    const double * y,
    double * gx,
    double * gy);

/* Vectorized geodetic coordinates */
void mulder_layer_geodetic_v(
    const struct mulder_layer * layer,
    int n,
    const double * x,
    const double * y,
    double * latitude,
    double * longitude);

/* Vectorized map coordinates */
void mulder_layer_coordinates_v(
    const struct mulder_layer * layer,
    int n,
    const double * latitude,
    const double * longitude,
    double * x,
    double * y);

/* Create a Turtle map from raw data */
void mulder_map_create(
    const char * path,
    const char * projection,
    int nx,
    int ny,
    double xmin,
    double xmax,
    double ymin,
    double ymax,
    const double * z);

/* Vectorized flux computation */
void mulder_fluxmeter_flux_v(
    struct mulder_fluxmeter * fluxmeter,
    double latitude,
    double longitude,
    double height,
    double azimuth,
    double elevation,
    int n,
    const double * energy,
    double * flux);

/* Vectorized reference flux */
void mulder_fluxmeter_reference_flux_v(
    struct mulder_fluxmeter * fluxmeter,
    double elevation,
    int n,
    const double * energy,
    double * flux);

/* Vectorized intersections */
void mulder_fluxmeter_intersect_v(
    struct mulder_fluxmeter * fluxmeter,
    double latitude,
    double longitude,
    double height,
    int n,
    const double * azimuth,
    const double * elevation,
    int * layer,
    double * x,
    double * y,
    double * z);

/* Vectorized gramage */
void mulder_fluxmeter_grammage_v(
    struct mulder_fluxmeter * fluxmeter,
    double latitude,
    double longitude,
    double height,
    int n,
    const double * azimuth,
    const double * elevation,
    double * grammage);

/* Vectorized locator */
void mulder_fluxmeter_whereami_v(
    struct mulder_fluxmeter * fluxmeter,
    int n,
    const double * latitude,
    const double * longitude,
    const double * height,
    int * layer);
