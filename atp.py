import yaml
import numpy as np
from astroquery.mast import Catalogs
import astropy.units as units
from astropy.units import Quantity
from astropy.time import Time
from astropy.coordinates import SkyCoord, EarthLocation, ICRS, AltAz, Angle
import logging

logging.basicConfig()
import ceo
from datetime import datetime

RAD2MAS = 180 * 3600e3 / np.pi
RAD2ARCSEC = 180 * 3600 / np.pi
ARCMIN2RAD = np.pi / 180 / 60


def Rz(c):
    return np.array([[np.cos(c), np.sin(c), 0], [-np.sin(c), np.cos(c), 0], [0, 0, 1]])


def Ry(b):
    return np.array([[np.cos(b), 0, -np.sin(b)], [0, 1, 0], [np.sin(b), 0, np.cos(b)]])


def Rx(a):
    return np.array([[1, 0, 0], [0, np.cos(a), np.sin(a)], [0, -np.sin(a), np.cos(a)]])


def photon_noise_variance(fwhm, Nph):
    return 0.5 * fwhm**2 / Nph / np.log(2)


def readout_noise_variance(Nph, ron2, p, Ns2, Nbg=0):
    sig2 = ron2 + Nbg
    return sig2 * (p * Ns2 / Nph) ** 2 / 12


def r0_scaling(atm_wavelength, gs_wavelength, zenith_distance=0):
    return (gs_wavelength / atm_wavelength) ** 1.2 * np.cos(zenith_distance) ** 0.6


def tt7_tt_error(zz, magnitude, zenith_distance, **kwargs):
    gs_wavelength = Quantity(*kwargs["TT7"]["guide star"]["wavelength"]).to("m").value
    r0_wavelength = Quantity(*kwargs["Atmosphere"]["wavelength"]).to("m").value
    r0 = Quantity(*kwargs["Atmosphere"]["r0"]).to("m").value  # *\
    # (gs_wavelength/r0_wavelength)**1.2
    # r0 = (r0**(-5./3.)/np.cos(zenith_distance))**(-3./5.)
    r0 *= r0_scaling(r0_wavelength, gs_wavelength, zenith_distance)
    D = Quantity(*kwargs["Telescope"]["diameter"]).to("m").value
    L0 = Quantity(*kwargs["Atmosphere"]["L0"]).to("m").value
    altitude = np.array(Quantity(*kwargs["Atmosphere"]["altitude"]).to("m").value)
    fr0 = np.array(kwargs["Atmosphere"]["fr0"])
    anisop = tilt_anisoplanatism(
        ARCMIN2RAD * zz, r0, gs_wavelength, L0, D, altitude, fr0
    )

    seeingArcsec = gs_wavelength / r0
    wfs_photoelectron_gain = (
        kwargs["TT7"]["optics"]["throughput"]
        * kwargs["TT7"]["detector"]["quantum efficiency"]
    )
    gs_nPhoton = kwargs["TT7"]["guide star"]["zero point"] * 10 ** (-0.4 * magnitude)
    nPhLenslet = (
        Quantity(*kwargs["TT7"]["detector"]["exposure"]).to("s").value
        * wfs_photoelectron_gain
        * gs_nPhoton
        * kwargs["Telescope"]["area"]
        / kwargs["TT7"]["optics"]["lenslet"]["array"]
    )
    pn = photon_noise_variance(seeingArcsec, nPhLenslet)
    ron2 = kwargs["TT7"]["detector"]["read-out noise"] ** 2
    rn = readout_noise_variance(nPhLenslet, ron2, 0.4 * ceo.constants.ARCSEC2RAD, 144)
    return np.sqrt(anisop + 2 * pn) * RAD2MAS


from scipy.special import j0, j1, jn, gamma
from scipy.integrate import quad


def tilt_anisoplanatism(zz, r0, wl, L0, D, altitude, fr0):
    def G(_f_, D):
        f = np.array(_f_)
        out = np.ones_like(f)
        idx = f != 0
        red = np.pi * D * f[idx]
        out[idx] = (2 * j1(red) / red) ** 2
        return out

    def integrandSF(f, theta, r0, wl, L0, D, altitude, fr0):
        f0 = 1.0 / L0
        cst = (
            np.pi
            * gamma(11.0 / 6.0) ** 2
            / (2 * np.pi ** (11.0 / 3.0))
            * (24 * gamma(6.0 / 5.0) / 5) ** (5.0 / 6.0)
        )
        cst *= wl**2 * r0 ** (-5.0 / 3.0)
        sum = 0.0
        for k in range(altitude.size):
            rho = theta * altitude[k]
            red = 2 * np.pi * rho * f
            sum += (
                fr0[k]
                * f**3
                * (f**2 + f0**2) ** (-11.0 / 6.0)
                * G(f, D)
                * (1 - j0(red))
            )
        return 4 * cst * sum

    cexy_var, err = quad(
        integrandSF,
        0,
        np.inf,  # limit=200,
        args=(zz, r0, wl, L0, D, altitude, fr0),
    )
    return cexy_var


def SH_GSs(
    probes, stars, zenith_distance, min_radius=6, nTest=10, nSample=10, **kwargs
):
    print("SH GSs...")
    mask0 = probes[0].stars_idx
    mask0 = np.logical_or(mask0, probes[1].stars_idx)
    mask0 = np.logical_or(mask0, probes[2].stars_idx)
    stars_xy = stars.local[:2, mask0]
    R = stars.V[mask0]
    stars_r = np.sqrt(np.sum(stars_xy**2, 0))
    stars_o = np.arctan2(stars_xy[1, :], stars_xy[0, :])
    q = Quantity(stars_r, "rad").to("arcmin")
    mask = np.logical_and(
        q > Quantity(min_radius, "arcmin"), q <= Quantity(10, "arcmin")
    )
    lo = 2 * np.pi / 3
    Rmin = []
    az_dist = []
    q = stars_o[mask]
    for c, _q_ in enumerate(q):
        qc = q - _q_
        id2 = [np.argmin(np.abs(qc + lo)), np.argmin(np.abs(qc - lo))]
        az_dist += [np.abs(qc[id2].sum())]
        Rmin += [np.min(R[mask][[c] + id2])]
    az_idx = np.argsort(az_dist)
    N_AST = az_idx.size
    if N_AST < nTest:
        nTest = N_AST
    print(f"# of possible triplets: {N_AST}")
    L = 25.5
    nPx = 201
    nLenslet = 48
    gmt = ceo.GMT_MX()
    src = ceo.Source(
        photometric_band="V",
        rays_box_size=L,
        rays_box_sampling=nPx,
        rays_origin=[0, 0, 25],
    )
    src >> (gmt,)

    gs_wavelength = Quantity(*kwargs["SH"]["guide star"]["wavelength"]).to("m").value
    r0_wavelength = Quantity(*kwargs["Atmosphere"]["wavelength"]).to("m").value
    r0 = Quantity(*kwargs["Atmosphere"]["r0"]).to("m").value
    r0 *= r0_scaling(r0_wavelength, gs_wavelength, zenith_distance)
    seeingArcsec = gs_wavelength / r0
    print(f"seeing: {seeingArcsec * ceo.constants.RAD2ARCSEC}arcsec")

    wfs_photoelectron_gain = (
        kwargs["SH"]["optics"]["throughput"]
        * kwargs["SH"]["detector"]["quantum efficiency"]
    )
    gs_nPhoton = kwargs["SH"]["guide star"]["zero point"]
    nPhLenslet0 = (
        Quantity(*kwargs["SH"]["detector"]["exposure"]).to("s").value
        * wfs_photoelectron_gain
        * gs_nPhoton
        * (
            Quantity(*kwargs["Telescope"]["diameter"]).to("m").value
            / kwargs["TT7"]["optics"]["lenslet"]["array"]
        )
        ** 2
    )
    px_scale = 0.4 * ceo.constants.ARCSEC2RAD
    ron2 = kwargs["SH"]["detector"]["read-out noise"] ** 2

    median_wfe_rms = []
    for kTest in range(nTest):
        id1 = az_idx[kTest]
        qc = q - q[id1]
        id2 = [np.argmin(np.abs(qc + lo)), np.argmin(np.abs(qc - lo))]
        ids = [id1] + id2

        wfs = ceo.GeometricShackHartmann(nLenslet, L / nLenslet, 3)
        zen = stars_r[mask][ids]
        azi = stars_o[mask][ids]
        gs = ceo.Source(
            photometric_band="R+I",
            zenith=zen.tolist(),
            azimuth=azi.tolist(),
            magnitude=R[mask][ids],
            rays_box_size=L,
            rays_box_sampling=nLenslet * 8 + 1,
            rays_origin=[0, 0, 25],
        )
        gs.reset()
        gmt.reset()
        gmt.propagate(gs)
        wfs.calibrate(gs, 0.0)
        gs >> (gmt, wfs)
        C = gmt.AGWS_calibrate(
            wfs,
            gs,
            decoupled=True,
            fluxThreshold=0.5,
            includeBM=False,
            filterMirrorRotation=True,
            calibrationVaultKwargs={
                "n_threshold": [2] * 6 + [0],
                "insert_zeros": [None] * 6 + [[5, 10]],
            },
        )

        wfe_rms = np.zeros(nSample)
        for k in range(nSample):
            n = np.random.randn(nLenslet**2 * 2, 3)  #
            for l in range(3):
                nPhLenslet = nPhLenslet0 * 10 ** (-0.4 * gs.magnitude[l])
                rms_noise = np.sqrt(
                    photon_noise_variance(seeingArcsec, nPhLenslet)
                    + readout_noise_variance(nPhLenslet, ron2, px_scale, 64)
                )

                n[:, l] *= rms_noise
                # print(f"V={gs.magnitude[l]},Nph={nPhLenslet},n={rms_noise*ceo.constants.RAD2MAS}")

            ~gmt
            state = gmt.state
            c = C.dot(n.reshape(-1, 1)).reshape(7, -1)

            state["M1"]["Txyz"] -= c[:, :3]
            state["M1"]["Rxyz"] -= c[:, 3:6]
            state["M2"]["Txyz"] -= c[:, 6:9]
            state["M2"]["Rxyz"] -= c[:, 9:12]

            gmt ^= state

            +src
            wfe_rms[k] = src.wavefront.rms(-9)

        median_wfe_rms += [np.median(wfe_rms)]
    print("MEDIAN WFE RMS [nm]:")
    print(median_wfe_rms)
    w = np.argsort(median_wfe_rms)[0]
    id1 = az_idx[w]
    qc = q - q[id1]
    id2 = [np.argmin(np.abs(qc + lo)), np.argmin(np.abs(qc - lo))]
    ids = [id1] + id2
    # print("ids:",ids)
    # print("R,O,V:",stars_r[mask][ids]*180*60/np.pi,stars_o[mask][ids]*180/np.pi,R[mask][ids])
    _probe_id = []
    for uid in np.where(mask0)[0][mask][ids]:
        # print('UID:',uid)
        dist = np.argsort(
            np.hstack(
                [
                    stars.distanceFrom(
                        Quantity(probe.local, "rad"), star_idx=[uid], u="arcmin"
                    ).value
                    for probe in probes
                ]
            )
        )
        for d in dist:
            if not d in _probe_id:
                probe_id = d
                _probe_id += [probe_id]
                break
        # print("PID:",probe_id)
        probes[probe_id].gs_idx = uid


class Observatory:
    def __init__(self, **kwargs):
        self.location = EarthLocation(
            lat=Quantity(*kwargs["latitude"]),
            lon=Quantity(*kwargs["longitude"]),
            height=Quantity(*kwargs["height"]),
        )
        self.time_scale = kwargs["time scale"]
        self.start_time = Time(kwargs["time"])
        self.current_time = Time(kwargs["time"])
        if self.current_time is None:
            self.current_time = Time(datetime.now())
        self.time_resolution = Quantity(*kwargs["time resolution"])
        # print(str(self))

    def __str__(self):
        return (
            "@(Observatory)> Location: "
            + str(self.location.geodetic)
            + "\n@(Observatory)> Time: "
            + str(self.current_time)
        )

    def update(self):
        self.current_time += self.time_resolution

    @property
    def frame(self):
        return AltAz(obstime=self.current_time, location=self.location)

    @property
    def sidereal_time(self):
        if self.time_scale == "LST":
            return Angle(self.current_time.value.split("T")[-1], unit="hourangle")
        else:
            return self.current_time.sidereal_time("apparent", self.location.lon)


class Target:
    def __init__(self, obs, **kwargs):
        self.obs = obs
        if kwargs["pointing target"] not in [None, "None"]:
            name = kwargs["pointing target"]
            try:
                self.icrs = SkyCoord.from_name(name, frame="icrs")
            except:
                s = name.split(",")
                print(s)
                if len(s) > 1:
                    kwargs["pointing ra/dec"] = {
                        "ra": [float(s[0][1:]), "degree"],
                        "dec": [float(s[1][:-1]), "degree"],
                    }
                    kwargs["pointing alt/az"] = None
        elif kwargs["pointing alt/az"] is not None:
            self.altaz = SkyCoord(
                alt=Quantity(*kwargs["pointing alt/az"]["alt"]),
                az=Quantity(*kwargs["pointing alt/az"]["az"]),
                frame=obs.frame,
            )
            self.icrs = self.altaz.transform_to(ICRS())
        if kwargs["pointing ra/dec"] is not None:
            self.icrs = SkyCoord(
                ra=Quantity(*kwargs["pointing ra/dec"]["ra"]),
                dec=Quantity(*kwargs["pointing ra/dec"]["dec"]),
                frame="icrs",
            )
        self.rotator_angle = Quantity(*kwargs["rotator angle"])
        self.update(rotator=False)
        # print("@(Target)>")
        # print(self.icrs)
        # print(self.altaz)

    def update(self, rotator=True):
        self.altaz = self.icrs.transform_to(self.obs.frame)
        H = (self.obs.sidereal_time.to(units.rad) - self.icrs.ra.to(units.rad)).value
        D = self.icrs.dec.to(units.rad).value
        L = self.obs.location.lat.to(units.rad).value
        self.parallactic_angle = Quantity(
            np.arctan2(
                np.sin(L) * np.cos(D) - np.cos(L) * np.cos(H) * np.sin(D),
                -np.cos(L) * np.sin(H),
            ),
            "deg",
        )
        if rotator:
            self.rotator_angle += (
                np.cos(self.obs.location.lat)
                * np.cos(self.altaz.az)
                / np.cos(self.altaz.alt)
                * self.obs.time_resolution.to("s").value
                * Quantity(15, "degree")
                / 3600
            )


class StarField:
    def __init__(self, obs, target, field=None, **kwargs):
        self.logger = logging.getLogger((self.__class__.__name__))
        self.logger.setLevel(logging.INFO)
        self.obs = obs
        self.target = target
        self.Vmag_lim = kwargs["V magnitude limit"]
        try:
            self.exclude_radius = Quantity(*kwargs["exclude radius"])
        except (TypeError, KeyError):
            self.logger.warning("No exclude radius set!")
            self.exclude_radius = None
        self.color = kwargs["color"]
        if field is None:
            radius = Quantity(*kwargs["radius"])
            self.logger.info("Querying TIC ...")
            # self.catalogData = Catalogs.query_region(
            #     target.icrs, radius=radius, catalog="TIC", objType="STAR"
            # )
            self.catalogData = Catalogs.query_criteria(
                catalog="TIC",
                coordinates=f"{target.icrs.ra.deg}, {target.icrs.dec.deg}",
                radius=radius,
                objType="STAR",
            )

            data = self.catalogData

            bd = []
            bdisp = []
            nrstars = len(data['disposition'])
            print("nrstars:", nrstars)
            for i in range(nrstars):
                if (
                    str(data["disposition"][i]) != "--"
                    and str(data["disposition"][i]) != "SPLIT"
                ):
                    bd.append(i)
                    bdisp.append(str(data["disposition"][i]))
            nbd = len(bd)
            if nbd > 0:
                data.remove_rows(bd)

            self.icrs = SkyCoord(
                ra=data["ra"] * units.deg, dec=data["dec"] * units.deg, frame="icrs"
            )
            self.update()
            self.apply_constrains()
            self.update()
        else:
            with open(field) as fp:
                data = yaml.safe_load(fp)
            self.icrs = SkyCoord(
                ra=Quantity(*data["ra"]), dec=Quantity(*data["dec"]), frame="icrs"
            )
            self.V = data["Vmag"]
            self.J = data["Jmag"]

            self.update(obs)

    def apply_constrains(self):
        self.logger.info(f" Catalog entry #{len(self.catalogData)}")
        if self.exclude_radius is not None:
            # uu = self.catalogData['ra']*units.deg - self.target.icrs.ra
            # vv = self.catalogData['dec']*units.deg - self.target.icrs.dec
            # mask = np.hypot(uu,vv) > self.exclude_radius
            mask = self.distanceFrom(u="arcmin") > self.exclude_radius
            self.logger.info("Entries accessible #%d", mask.sum())
        else:
            mask = np.ones(len(self.catalogData), dtype=bool)
        self.logger.info(f"Exclude radius: {mask.sum()}")
        _mask_ = np.ones_like(mask, dtype=bool)
        for c in self.color:
            c_mag = np.array(self.catalogData[f"{c}mag"])
            _mask_ = np.logical_and(_mask_, ~np.isnan(c_mag))
        mask = np.logical_and(mask, _mask_)
        self.logger.info(f"Color: {mask.sum()}")
        Vmag = self.catalogData["Vmag"]
        Vmag[np.isnan(Vmag)] = self.Vmag_lim + 1
        _mask_ = Vmag <= self.Vmag_lim
        mask = np.logical_and(mask, _mask_)
        self.logger.info(f"Magnitude: {mask.sum()}")
        data = self.catalogData[mask]
        for c in self.color:
            setattr(self, c, data[f"{c}mag"])
        n = mask.sum()
        self.mask = mask
        self.logger.info(f"Entries with {self.color} magnitude #{n}")
        if n > 0:
            for c in self.color:
                self.logger.info(
                    "V magnitude range [{0},{1}]".format(
                        getattr(self, c).min(), getattr(self, c).max()
                    )
                )
            self.icrs = SkyCoord(
                ra=data["ra"] * units.deg, dec=data["dec"] * units.deg, frame="icrs"
            )
            self.update()
        else:
            print("No stars meet the criteria!")
        return n

    def update(self, *args):
        self.altaz = self.icrs.transform_to(self.obs.frame)
        cra = np.cos(self.altaz.az.to(units.rad))
        sra = np.sin(self.altaz.az.to(units.rad))
        cdec = np.cos(self.altaz.alt.to(units.rad))
        sdec = np.sin(self.altaz.alt.to(units.rad))
        x = cra * cdec
        y = sra * cdec
        z = sdec
        # print(self.target.rotator_angle)
        R = (
            Rz(self.target.rotator_angle)
            @ Ry(np.pi / 2 * units.rad - self.target.altaz.alt.to(units.rad))
            @ Rz(self.target.altaz.az.to(units.rad))
        )
        v = np.array(np.vstack([x, y, z]))
        self.local = R @ v

    def distanceFrom(
        self, origin=Quantity([[0.0], [0.0]], "arcmin"), star_idx=np.s_[:], u=None
    ):
        dist = np.sqrt(
            np.sum((self.local[:2, star_idx] - origin.to(units.rad).value) ** 2, 0)
        )
        if u is not None:
            dist = Quantity(dist, units.rad).to(u)
        return dist


class Probe:
    def __init__(self, az, exclude_rad=Quantity(2, "arcmin")):
        self.rad = Quantity(1360 / 60, "arcmin")
        self.az = az
        self.range_rad = self.rad - exclude_rad
        self.local = np.array(
            [
                [self.rad.to("rad").value * np.cos(self.az).value],
                [self.rad.to("rad").value * np.sin(self.az).value],
            ]
        )
        self.gs_idx = None

    def reachForTheStars(self, stars):
        dist_to_probe = stars.distanceFrom(Quantity(self.local, "rad"))
        self.stars_idx = dist_to_probe <= self.range_rad.to("rad").value


if __name__ == "__main__":
    CFG_FILE = "atp.yaml"
    with open(CFG_FILE) as fp:
        cfg = yaml.safe_load(fp)
    obs = Observatory(**cfg["Observatory"], **cfg["Observation"])
    target = Target(obs, **cfg["Target"])
    stars = StarField(obs, target, **cfg["Star Catalog"])
    probes = [Probe(Quantity(k * 90, "degree")) for k in range(4)]
    for k in range(4):
        probes[k].reachForTheStars(stars)
    zen = np.pi / 2 - target.altaz.alt.to("rad").value
    tt_res_rms = [
        tt7_tt_error(zz, magnitude, zen, **cfg)
        for zz, magnitude in zip(stars.distanceFrom(u="arcmin").value, stars.V)
    ]
    print(tt_res_rms)
    tt7_gs_idx = np.argmin(tt_res_rms)
    tt7_dist = []
    for k in range(4):
        tt7_dist += [
            stars.distanceFrom(
                Quantity(probes[k].local, "rad"), star_idx=[tt7_gs_idx], u="arcmin"
            )
        ]
    tt7_idx = np.argmin(tt7_dist)
    TT7 = probes[tt7_idx]
    TT7.gs_idx = tt7_gs_idx
    probes.pop(tt7_idx)
    SH_GSs(probes, stars, zen, **cfg)
