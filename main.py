from bokeh.io import curdoc
from bokeh.plotting import figure, show, output_file
from bokeh.models import (
    ColorBar,
    LinearColorMapper,
    BasicTicker,
    ColumnDataSource,
    PrintfTickFormatter,
    HoverTool,
)
from bokeh.palettes import Plasma11, Spectral4
from bokeh.layouts import row, column, layout
from bokeh.models.widgets import (
    Button,
    Slider,
    TextInput,
    Toggle,
)
import numpy as np
import yaml
from astropy.units import Quantity
from atp import Observatory, Target, StarField, Probe, tt7_tt_error, SH_GSs


class Model:
    def __init__(self):
        self.obs = None
        self.target = None
        self.stars = None
        self.TT7 = None
        self.probes = [Probe(Quantity(k * 90, "degree")) for k in range(4)]


class CDS:
    def __init__(self):
        self.stars = ColumnDataSource(data=dict(x=[], y=[], Vmag=[], c=[], Jmag=[]))
        self.probes = {"mirror": [], "stage": []}
        for k in range(4):
            self.probes["mirror"] += [ColumnDataSource(data=dict(tx=[], ty=[], lc=[]))]
            self.probes["stage"] += [ColumnDataSource(data=dict(px=[], py=[]))]


CFG_FILE = __file__.replace("main.py", "atp.yaml")
with open(CFG_FILE) as fp:
    cfg = yaml.safe_load(fp)

r2a = 180 * 60 / np.pi
mdl = Model()
cds = CDS()

doc = curdoc()

startstop = Toggle(label="Start/Stop Time", button_type="default", active=False)


def update():
    source = cds.stars
    probes = [mdl.TT7] + mdl.probes
    mdl.obs.update()
    mdl.target.update(rotator=rotator.active)
    mdl.stars.update()
    x = r2a * mdl.stars.local[0, :]
    y = r2a * mdl.stars.local[1, :]
    source.data.update(dict(x=x, y=y))
    p.title.text = (
        "AGWS - "
        + str(mdl.obs.current_time)
        + " - Alt: "
        + str(mdl.target.altaz.alt)
        + ",Az: "
        + str(mdl.target.altaz.az)
    )
    for probe, mirror, stage in zip(probes, cds.probes["mirror"], cds.probes["stage"]):
        """
        p.line([r2a*probe.local[0,0],r2a*mdl.stars.local[0,probe.gs_idx]],
               [r2a*probe.local[1,0],r2a*mdl.stars.local[1,probe.gs_idx]],
               line_width=5,color='navy')
        p.scatter(r2a*mdl.stars.local[0,probe.gs_idx],r2a*mdl.stars.local[1,probe.gs_idx],
                 size=10,fill_color=None,line_color='DarkSeaGreen',line_width=3)
        """
        src = ColumnDataSource(
            data=dict(
                tx=[r2a * mdl.stars.local[0, probe.gs_idx]],
                ty=[r2a * mdl.stars.local[1, probe.gs_idx]],
                lc=["DarkSeaGreen"],
            )
        )
        data = dict(
            tx=[r2a * mdl.stars.local[0, probe.gs_idx]],
            ty=[r2a * mdl.stars.local[1, probe.gs_idx]],
        )
        mirror.data.update(data)
        data = dict(
            px=[r2a * probe.local[0, 0], r2a * mdl.stars.local[0, probe.gs_idx]],
            py=[r2a * probe.local[1, 0], r2a * mdl.stars.local[1, probe.gs_idx]],
        )
        stage.data.update(data)


widget_data = {"startstop": None}
callback_id = None


def cb_startstop(attrname, old, new):
    global callback_id
    print(attrname, old, new)
    if new:
        print("Add callback")
        callback_id = doc.add_periodic_callback(update, 1000)
    else:
        print("Remove callback")
        doc.remove_periodic_callback(callback_id)


startstop.on_change("active", cb_startstop)
datetime = TextInput(value="2018-01-01T04:00:00.000", title="Date and time (UTC)")
time_res = TextInput(value="60", title="Time resolution [s]")
target_name = TextInput(value="None", title="Target name or coordinates (ra,dec)")
tel_alt = TextInput(value="45", title="Telescope altitude [degree]")
tel_az = TextInput(value="0", title="Telescope azimuth [degree]")
query = Button(label="Query field", button_type="default")


def cb_query():
    mdl.obs = Observatory(
        **{
            "time": datetime.value,
            "time scale": "UTC",
            "time resolution": [float(time_res.value), "second"],
        },
        **cfg["Observatory"],
    )
    mdl.target = Target(
        mdl.obs,
        **{
            "pointing target": target_name.value,
            "pointing ra/dec": None,
            "pointing alt/az": {
                "alt": [float(tel_alt.value), "degree"],
                "az": [float(tel_az.value), "degree"],
            },
            "rotator angle": [0, "degree"],
        },
    )
    if target_name.value == "None":
        target_name.value = "({0:.2f},{1:.2f})".format(
            mdl.target.icrs.ra.value, mdl.target.icrs.dec.value
        )
    mdl.probes = [Probe(Quantity(k * 90, "degree")) for k in range(4)]
    tel_alt.value = "%.2f" % mdl.target.altaz.alt.value
    tel_az.value = "%.2f" % mdl.target.altaz.az.value
    mdl.stars = StarField(
        mdl.obs,
        mdl.target,
        **cfg[
            "Star Catalog"
        ],  # **{"V magnitude limit": 18, "exclude radius": [3, "arcmin"], "color": ["V", "J"]},
    )
    Vmag = mdl.stars.V  # [idx]
    Jmag = mdl.stars.J  # [idx]
    q = np.rint((Vmag - Vmag.min()) / (Vmag.max() - Vmag.min()) * 10)
    c = [Plasma11[int(x)] for x in q]
    x = r2a * mdl.stars.local[0, :]
    y = r2a * mdl.stars.local[1, :]
    src = ColumnDataSource(
        data=dict(
            x=x.tolist(),
            y=y.tolist(),
            Vmag=Vmag.tolist(),
            c=c,
            Jmag=Jmag.tolist(),
        )
    )
    cds.stars.data.update(src.data)
    p.title.text = (
        "AGWS - "
        + str(mdl.obs.current_time)
        + " - Alt: "
        + str(mdl.target.altaz.alt)
        + ",Az: "
        + str(mdl.target.altaz.az)
    )
    V_lim.start = np.floor(mdl.stars.V.min())
    V_lim.end = np.ceil(mdl.stars.V.max())
    V_lim.value = mdl.stars.Vmag_lim


query.on_click(cb_query)
find_tt7 = Button(label="Find GSs", button_type="default")


def cb_find_tt7():
    for k in range(4):
        mdl.probes[k].reachForTheStars(mdl.stars)
    zen = np.pi / 2 - mdl.target.altaz.alt.to("rad").value
    tt_res_rms = [
        tt7_tt_error(zz, magnitude, zen, **cfg)
        for zz, magnitude in zip(mdl.stars.distanceFrom(u="arcmin").value, mdl.stars.V)
    ]
    print(tt_res_rms)
    tt7_gs_idx = np.argmin(tt_res_rms)
    tt7_dist = []
    for k in range(4):
        tt7_dist += [
            mdl.stars.distanceFrom(
                Quantity(mdl.probes[k].local, "rad"), star_idx=[tt7_gs_idx], u="arcmin"
            )
        ]
    tt7_idx = np.argmin(tt7_dist)
    mdl.TT7 = mdl.probes[tt7_idx]
    mdl.TT7.gs_idx = tt7_gs_idx
    mdl.probes.pop(tt7_idx)
    """
    p.line([r2a*mdl.TT7.local[0,0],r2a*mdl.stars.local[0,mdl.TT7.gs_idx]],
           [r2a*mdl.TT7.local[1,0],r2a*mdl.stars.local[1,mdl.TT7.gs_idx]],
           line_width=5,color='navy')
    p.circle(r2a*mdl.stars.local[0,mdl.TT7.gs_idx],r2a*mdl.stars.local[1,mdl.TT7.gs_idx],
             size=10,fill_color=None,line_color='FireBrick',line_width=3)
    """
    SH_GSs(mdl.probes, mdl.stars, zen, **cfg)

    probes = [mdl.TT7] + mdl.probes
    for probe, mirror, stage, c in zip(
        probes,
        cds.probes["mirror"],
        cds.probes["stage"],
        ["FireBrick"] + ["DarkSeaGreen"] * 3,
    ):
        """
        p.line([r2a*probe.local[0,0],r2a*mdl.stars.local[0,probe.gs_idx]],
               [r2a*probe.local[1,0],r2a*mdl.stars.local[1,probe.gs_idx]],
               line_width=5,color='navy')
        p.scatter(r2a*mdl.stars.local[0,probe.gs_idx],r2a*mdl.stars.local[1,probe.gs_idx],
                 size=10,fill_color=None,line_color='DarkSeaGreen',line_width=3)
        """
        src = ColumnDataSource(
            data=dict(
                tx=[r2a * mdl.stars.local[0, probe.gs_idx]],
                ty=[r2a * mdl.stars.local[1, probe.gs_idx]],
                lc=[c],
            )
        )
        mirror.data.update(src.data)
        src = ColumnDataSource(
            data=dict(
                px=[r2a * probe.local[0, 0], r2a * mdl.stars.local[0, probe.gs_idx]],
                py=[r2a * probe.local[1, 0], r2a * mdl.stars.local[1, probe.gs_idx]],
            )
        )
        stage.data.update(src.data)


find_tt7.on_click(cb_find_tt7)
rotator = Toggle(label="Rotator On/Off", button_type="default", active=False)
V_lim = Slider(start=0, end=18, value=18, step=1, title="V magnitude limit")


def cb_V_lim(attrname, old, new):
    mdl.stars.Vmag_lim = new
    mdl.stars.apply_constrains()
    # for k in range(4):
    #    mdl.probes[k].reachForTheStars(mdl.stars)
    Vmag = mdl.stars.V  # [idx]
    Jmag = mdl.stars.J  # [idx]
    q = np.rint((Vmag - Vmag.min()) / (Vmag.max() - Vmag.min()) * 10)
    c = [Plasma11[int(x)] for x in q]
    src = ColumnDataSource(
        data=dict(
            x=r2a * mdl.stars.local[0, :],
            y=r2a * mdl.stars.local[1, :],
            Vmag=Vmag,
            c=c,
            Jmag=Jmag,
        )
    )
    cds.stars.data.update(src.data)


V_lim.on_change("value", cb_V_lim)

p = figure(
    title="AGWS",
    x_range=[-25, 25],
    y_range=[-25, 25],
    tools="pan,wheel_zoom,box_zoom,reset",
)
for k in range(4):
    p.line(x="px", y="py", source=cds.probes["stage"][k], line_width=3, color="navy")
    p.circle(
        x="tx",
        y="ty",
        line_color="lc",
        source=cds.probes["mirror"][k],
        radius=0.5,
        fill_color=None,
        line_width=2,
    )
    p.wedge(
        r2a * mdl.probes[k].local[0, 0],
        r2a * mdl.probes[k].local[1, 0],
        radius=21,
        start_angle=(k * 90 + 180 - 26.35) * np.pi / 180,
        end_angle=(k * 90 + 180 + 26.35) * np.pi / 180,
        color="navy",
        alpha=0.05,
    )
    p.scatter(
        r2a * mdl.probes[k].local[0, 0],
        r2a * mdl.probes[k].local[1, 0],
        size=15,
        color="navy",
    )
stars = p.scatter(x="x", y="y", color="c", size=5, source=cds.stars, alpha=0.75)
stars.name = "stars"
p.circle(0, 0, radius=3, fill_color=None, line_color="red", line_dash="dashed")
p.circle(0, 0, radius=10, fill_color=None, line_color="red", line_dash="dashed")
p.xaxis.axis_label = "[arcmin]"
p.yaxis.axis_label = "[arcmin]"
p.add_tools(HoverTool(renderers=[stars], tooltips=[("V", "@Vmag"), ("J", "@Jmag")]))

doc.add_root(
    row(
        column(
            [
                datetime,
                target_name,
                tel_alt,
                tel_az,
                query,
                V_lim,
                find_tt7,
                time_res,
                startstop,
                rotator,
            ]
        ),
        p,
    )
)
# update()
