"""Builds the Bytepet player's manual and the two-page quick reference card (PDF).

Every number and name in the tables comes from content/*.toml, and every picture is a
real render from the game, so both match the build they ship with:

    cargo run --example export_art --no-default-features -- art/
    python3 tools/build_manual.py art/ dist/

That writes Bytepet_Manual.pdf and Bytepet_Quick_Reference.pdf into dist/.
Needs Python 3.11+ and reportlab.
"""
import os
import re
import sys
import tomllib

from reportlab.lib import colors
from reportlab.lib.enums import TA_CENTER
from reportlab.lib.pagesizes import letter
from reportlab.lib.styles import ParagraphStyle
from reportlab.lib.units import inch
from reportlab.platypus import (BaseDocTemplate, CondPageBreak, Frame, Image, KeepTogether, NextPageTemplate,
                                PageBreak, PageTemplate, Paragraph, Spacer, Table, TableStyle)

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
IMG = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "art")
OUT_DIR = sys.argv[2] if len(sys.argv) > 2 else ROOT


def load(name):
    with open(os.path.join(ROOT, "content", name), "rb") as f:
        return tomllib.load(f)


GAME = load("game.toml")
CHARS = {c["id"]: c for c in load("characters.toml")["character"]}
VERSION = re.search(r'^version = "([^"]+)"', open(os.path.join(ROOT, "Cargo.toml")).read(), re.M).group(1)
LIFE, CARE, EVO = GAME["life"], GAME["care"], GAME["evolution"]
STAGES = GAME["stage"]
ITEMS = GAME["item"]
JOBS = GAME["job"]
MAIL = GAME["mail"]
FRIENDS = GAME["friend_level"]
# the age in days (as Status shows it) when a pet becomes an adult
ADULT_AGE = (LIFE["egg"] + LIFE["baby"] + LIFE["child"] + LIFE["teen"]) // 86400


def name(cid):
    return CHARS[cid]["name"].title() if cid in CHARS else "Egg"


def find(stage, path=None, grade=None):
    for c in CHARS.values():
        if c["stage"] == stage and c.get("path") == path and (grade is None or c.get("grade") == grade):
            return c["id"]
    raise KeyError((stage, path, grade))


def dur(s):
    """A duration in seconds as words: 15 minutes, 2 hours, 1 day."""
    for unit, n in (("day", 86400), ("hour", 3600), ("minute", 60)):
        if s >= n and s % n == 0:
            k = s // n
            return "%d %s%s" % (k, unit, "" if k == 1 else "s")
    return "%d minutes" % round(s / 60)


def mins(s):
    return "%d min" % (s // 60)


PINK = colors.HexColor("#EC4899")
DARK = colors.HexColor("#1C1F2A")
INK = colors.HexColor("#262A33")
MUTED = colors.HexColor("#6B7280")
SOFT = colors.HexColor("#FDF2F8")
RULE = colors.HexColor("#E5E7EB")

PW, PH = letter
M = 0.75 * inch
CW = PW - 2 * M

ss = {
    "h1": ParagraphStyle("h1", fontName="Helvetica-Bold", fontSize=22, leading=26, textColor=INK, spaceAfter=4),
    "num": ParagraphStyle("num", fontName="Helvetica-Bold", fontSize=10, leading=12, textColor=PINK, spaceAfter=2,
                          spaceBefore=26),  # dropped at the top of a page
    "h2": ParagraphStyle("h2", fontName="Helvetica-Bold", fontSize=13, leading=16, textColor=INK, spaceBefore=12,
                         spaceAfter=4),
    "body": ParagraphStyle("body", fontName="Helvetica", fontSize=10, leading=14.5, textColor=INK, spaceAfter=6),
    "lead": ParagraphStyle("lead", fontName="Helvetica", fontSize=11.5, leading=16, textColor=MUTED, spaceAfter=10),
    "cell": ParagraphStyle("cell", fontName="Helvetica", fontSize=9, leading=12, textColor=INK),
    "cellb": ParagraphStyle("cellb", fontName="Helvetica-Bold", fontSize=9, leading=12, textColor=INK),
    "th": ParagraphStyle("th", fontName="Helvetica-Bold", fontSize=8.5, leading=11, textColor=colors.white),
    "cap": ParagraphStyle("cap", fontName="Helvetica", fontSize=8, leading=10, textColor=MUTED, alignment=TA_CENTER),
    "capb": ParagraphStyle("capb", fontName="Helvetica-Bold", fontSize=8, leading=10, textColor=INK,
                           alignment=TA_CENTER),
    "tip": ParagraphStyle("tip", fontName="Helvetica", fontSize=9.5, leading=13.5, textColor=INK),
    "bullet": ParagraphStyle("bullet", fontName="Helvetica", fontSize=10, leading=14, textColor=INK, leftIndent=14,
                             bulletIndent=2, spaceAfter=3),
}


def P(text, style="body"):
    return Paragraph(text, ss[style])


def B(text):
    return Paragraph(text, ss["bullet"], bulletText="•")


def code(text):
    return "<font face='Courier'>%s</font>" % text


def img(nm, width):
    path = os.path.join(IMG, nm + ".png")
    im = Image(path)
    return Image(path, width=width, height=width * im.imageHeight / im.imageWidth)


def section(n, title, lead=None):
    out = [CondPageBreak(2.5 * inch), P("SECTION %d" % n, "num"), P(title, "h1")]
    if lead:
        out.append(P(lead, "lead"))
    return out


def table(rows, widths, header=True):
    data = []
    for r_i, row in enumerate(rows):
        data.append([Paragraph(c, ss["th"] if (header and r_i == 0) else ss["cell"]) if isinstance(c, str) else c
                     for c in row])
    t = Table(data, colWidths=widths, repeatRows=1 if header else 0)
    st = [("VALIGN", (0, 0), (-1, -1), "MIDDLE"),
          ("TOPPADDING", (0, 0), (-1, -1), 5), ("BOTTOMPADDING", (0, 0), (-1, -1), 5),
          ("LEFTPADDING", (0, 0), (-1, -1), 6), ("RIGHTPADDING", (0, 0), (-1, -1), 6),
          ("LINEBELOW", (0, 0), (-1, -1), 0.5, RULE)]
    if header:
        st.append(("BACKGROUND", (0, 0), (-1, 0), INK))
    for i in range(1 if header else 0, len(data)):
        if i % 2 == 0:
            st.append(("BACKGROUND", (0, i), (-1, i), colors.HexColor("#F9FAFB")))
    t.setStyle(TableStyle(st))
    return t


def tip(text, label="TIP"):
    t = Table([[Paragraph("<b>%s</b>&nbsp;&nbsp;%s" % (label, text), ss["tip"])]], colWidths=[CW])
    t.setStyle(TableStyle([("BACKGROUND", (0, 0), (-1, -1), SOFT), ("LINEBEFORE", (0, 0), (0, -1), 3, PINK),
                           ("LEFTPADDING", (0, 0), (-1, -1), 10), ("TOPPADDING", (0, 0), (-1, -1), 7),
                           ("BOTTOMPADDING", (0, 0), (-1, -1), 7)]))
    return t


def screens(items, per_row=3):
    """Rows of LCD captures with captions. items: (image, caption)."""
    w = (CW - (per_row - 1) * 10) / per_row
    rows = []
    for i in range(0, len(items), per_row):
        chunk = items[i:i + per_row]
        rows.append([img(n, w) for n, _ in chunk] + [""] * (per_row - len(chunk)))
        rows.append([P(c, "cap") for _, c in chunk] + [""] * (per_row - len(chunk)))
    t = Table(rows, colWidths=[w + 10] * per_row)
    t.setStyle(TableStyle([("ALIGN", (0, 0), (-1, -1), "CENTER"), ("LEFTPADDING", (0, 0), (-1, -1), 0),
                           ("RIGHTPADDING", (0, 0), (-1, -1), 0), ("TOPPADDING", (0, 0), (-1, -1), 2),
                           ("BOTTOMPADDING", (0, 0), (-1, -1), 4)]))
    return t


def char_cell(cid, w=0.72 * inch, sub=None):
    inner = [[img("char_" + cid, w)], [P(name(cid), "capb")]]
    if sub:
        inner.append([P(sub, "cap")])
    t = Table(inner, colWidths=[w + 8])
    t.setStyle(TableStyle([("ALIGN", (0, 0), (-1, -1), "CENTER"), ("LEFTPADDING", (0, 0), (-1, -1), 0),
                           ("RIGHTPADDING", (0, 0), (-1, -1), 0), ("TOPPADDING", (0, 0), (-1, -1), 1),
                           ("BOTTOMPADDING", (0, 0), (-1, -1), 1)]))
    return t


# ------------------------------------------------------------------ page decorations
def cover(c, doc):
    c.saveState()
    c.setFillColor(DARK)
    c.rect(0, 0, PW, PH, fill=1, stroke=0)
    c.setFillColor(PINK)
    c.rect(0, PH - 10, PW, 10, fill=1, stroke=0)
    title = Image(os.path.join(IMG, "title.png"))
    tw = 4.2 * inch
    c.drawImage(os.path.join(IMG, "title.png"), (PW - tw) / 2, PH - 1.9 * inch, width=tw,
                height=tw * title.imageHeight / title.imageWidth, mask="auto")
    c.setFillColor(colors.white)
    c.setFont("Helvetica", 15)
    c.drawCentredString(PW / 2, PH - 2.35 * inch, "Player's Manual")
    dev = Image(os.path.join(IMG, "device_cover.png"))
    dw = 3.7 * inch
    c.drawImage(os.path.join(IMG, "device_cover.png"), (PW - dw) / 2, 2.0 * inch, width=dw,
                height=dw * dev.imageHeight / dev.imageWidth, mask="auto")
    c.setFillColor(colors.HexColor("#A1A6B4"))
    c.setFont("Helvetica", 10)
    c.drawCentredString(PW / 2, 1.5 * inch, "A virtual pet that lives on your desktop · Windows, macOS and Linux")
    c.setFont("Helvetica", 8.5)
    c.drawCentredString(PW / 2, 0.75 * inch, "Version %s  ·  An original fan-made homage. Not affiliated with "
                                             "Bandai or the Tamagotchi brand." % VERSION)
    c.restoreState()


def normal(c, doc):
    c.saveState()
    c.setFillColor(PINK)
    c.rect(M, PH - 0.5 * inch, 22, 3, fill=1, stroke=0)
    c.setFont("Helvetica-Bold", 8)
    c.setFillColor(MUTED)
    c.drawString(M + 28, PH - 0.5 * inch - 1, "BYTEPET PLAYER'S MANUAL")
    c.setFont("Helvetica", 8)
    c.drawRightString(PW - M, 0.5 * inch, str(doc.page))
    c.restoreState()


# ------------------------------------------------------------------ content
story = [NextPageTemplate("body"), PageBreak()]

story += [P("Welcome", "h1"),
          P("Bytepet is a tiny monochrome pet that lives in a see-through, egg-shaped handheld floating on "
            "your desktop. Feed it, clean up after it, train it and play with it, and it grows from a blinking "
            "egg into one of %d different characters. Raise it well, find it a partner, and its egg starts the "
            "next generation." % len(CHARS), "lead")]
toc = [("1", "Getting started"), ("2", "The device and your desktop"), ("3", "Life cycle"), ("4", "Daily care"),
       ("5", "Skills and evolution"), ("6", "Mini-games"), ("7", "Mail, school and jobs"), ("8", "Town"),
       ("9", "Linking two Bytepets"), ("10", "Marriage and generations"), ("11", "Quick reference")]
tt = Table([[Paragraph("<font color='#EC4899'><b>%s</b></font>" % n, ss["body"]), Paragraph(t, ss["body"])]
            for n, t in toc], colWidths=[0.4 * inch, CW - 0.4 * inch])
tt.setStyle(TableStyle([("LINEBELOW", (0, 0), (-1, -1), 0.5, RULE), ("TOPPADDING", (0, 0), (-1, -1), 4),
                        ("BOTTOMPADDING", (0, 0), (-1, -1), 2), ("LEFTPADDING", (0, 0), (-1, -1), 0)]))
story += [Spacer(1, 6), tt, Spacer(1, 18),
          tip("Time in Bytepet runs at real-world speed and follows your computer's clock, and your pet keeps "
              "living while the app is closed. Check in a few times a day, just like the handhelds it is "
              "modelled on.", "NOTE"),
          PageBreak()]

# 1 getting started
story += section(1, "Getting started")
story += [P("Installing", "h2"),
          B("<b>Windows 10/11:</b> unzip %s anywhere (Documents is fine) and run %s. If SmartScreen warns "
            "about an unrecognised app, choose <i>More info → Run anyway</i>."
            % (code("bytepet-%s-windows-x86_64.zip" % VERSION), code("bytepet.exe"))),
          B("<b>macOS 11 or later:</b> unzip %s and drag %s into Applications. The first time, right-click it "
            "and choose <i>Open</i>; the app isn't notarized. It has no Dock icon, so look for it in the menu bar."
            % (code("bytepet-%s-macos-universal.zip" % VERSION), code("Bytepet.app"))),
          B("<b>Linux:</b> extract %s and run %s, then start Bytepet from your app menu. %s removes it again and "
            "keeps your pet." % (code("bytepet-%s-linux-x86_64.tar.gz" % VERSION), code("./install.sh"),
                                 code("./install.sh --remove"))),
          P("Your first egg", "h2"),
          P("Bytepet appears near the bottom-right corner of your screen; drag it wherever you like. First it "
            "asks for a name. Type it on your keyboard and press Enter, or use the buttons: <b>A</b> cycles "
            "through the letters, <b>B</b> adds the letter shown, <b>C</b> deletes, and choosing <b>END</b> "
            "finishes. Names are up to 7 letters; leave it blank for a random one. The egg hatches %s later."
            % dur(LIFE["egg"])),
          screens([("lcd_name", "Naming your pet"), ("lcd_egg", "The egg wobbles before hatching"),
                   ("lcd_main", "A happy child wandering the screen")])]

# 2 device
story += section(2, "The device and your desktop", "Everything is done with three buttons, just like the real "
                                                   "thing. Click them, or use the keyboard once Bytepet has focus.")
ctrl = table([["Button", "Keyboard", "What it does"],
              ["<b>A</b>", "Z, A, J, 1, ←", "Cycle through icons, menu items and choices"],
              ["<b>B</b>", "X, S, K, 2, ↓, Enter, Space",
               "Confirm. On the main screen with no icon selected: show the clock"],
              ["<b>C</b>", "C, D, L, 3, →, Esc", "Back / cancel. On the main screen: clear the selection "
                                                     "and call your pet back to the middle"],
              ["<b>A + C</b>", "M", "Mute or unmute the beeps"]],
             [0.6 * inch, 1.3 * inch, CW - 2.35 * inch - 1.9 * inch])
dev = Table([[img("size_medium", 2.0 * inch), ctrl]], colWidths=[2.35 * inch, CW - 2.35 * inch])
dev.setStyle(TableStyle([("VALIGN", (0, 0), (-1, -1), "MIDDLE"), ("LEFTPADDING", (0, 0), (-1, -1), 0)]))
story += [dev, Spacer(1, 4),
          P("Extra keys: <b>F1</b> help, <b>F2</b> change time speed, <b>F3</b> change shell design."),
          P("Playing with the mouse", "h2"),
          table([["Mouse", "What it does"],
                 ["Click A, B or C", "Presses that button"],
                 ["Click an icon", "Opens it, even from inside another menu"],
                 ["Click the screen", "B: choose"],
                 ["Scroll wheel", "Down: next item (A). Up: previous item."],
                 ["Right-click", "C: back, or clear the selected icon"],
                 ["Drag the shell or screen", "Moves Bytepet"]],
                [1.8 * inch, CW - 1.8 * inch])]

icon_info = [
    ("Status", "Name, age, weight, generation, hunger, happiness, training meter, the three skills, Gotchi "
               "Points (GP) and job. Press A or B to page through."),
    ("Food", "MEAL fills a hunger heart. SNACK fills a happiness heart but adds more weight. ITEMS uses food "
             "you bought in the shop."),
    ("Toilet", "Flushes away any poop."),
    ("Game", "The four mini-games, plus WORK once your pet has a job."),
    ("Lights", "Turn the lights off when your pet falls asleep, and back on if you like."),
    ("Medicine", "Treats sickness (the skull). Some illnesses need two doses."),
    ("Discipline", "SCOLD or PRAISE. See Daily care for when to use each."),
    ("Link", "Connect to another Bytepet on your network (section 9)."),
    ("Town", "Mail, shop, items, matchmaker, friends, album and setup."),
    ("Call", "Not selectable. Lights up when your pet needs you."),
]
rows = [["", "Icon", "What it does"]]
for i, (n, d) in enumerate(icon_info):
    rows.append([img("icon_%d" % i, 0.3 * inch), "<b>%s</b>" % n, d])
story += [P("The ten icons", "h2"),
          P("Five icons sit above the screen and five below. Press <b>A</b> to move the highlight and <b>B</b> to "
            "choose."),
          table(rows, [0.5 * inch, 0.95 * inch, CW - 1.45 * inch])]

sizes = Table([[img("size_small", 0.40 * 2.6 * inch), img("size_medium", 0.55 * 2.6 * inch),
                img("size_large", 0.75 * 2.6 * inch)],
               [P("Small", "cap"), P("Medium (default)", "cap"), P("Large", "cap")]],
              colWidths=[CW * 0.26, CW * 0.33, CW * 0.41])
sizes.setStyle(TableStyle([("ALIGN", (0, 0), (-1, -1), "CENTER"), ("VALIGN", (0, 0), (-1, 0), "BOTTOM")]))
menu = Table([[img("menu_open", 2.1 * inch),
               table([["Menu item", "What it does"],
                      ["SIZE", "Small, Medium, Large or Huge."],
                      ["SHELL", "Pick one of twelve designs, or change each part of the device yourself "
                                "(see section 8)."],
                      ["SPEED", "1× is real time; 10×, 60× and 600× are for trying "
                                "things out."],
                      ["SOUND", "The beeps: off, or 25%, 50%, 75% or 100% volume. A and C together mute and unmute."],
                      ["ON TOP", "Keep Bytepet above other windows."],
                      ["NOTIFY", "System notifications when your pet calls, gets sick, hatches, grows up, gets "
                                 "mail or leaves."],
                      ["AT LOGIN", "Start Bytepet automatically when you sign in."],
                      ["HELP", "The controls at a glance."],
                      ["HIDE TO TRAY", "Tuck it away; the tray (or menu-bar) icon brings it back. On Linux "
                                       "this reads MINIMIZE and minimizes instead."],
                      ["QUIT / CLOSE", "Close Bytepet (it saves first), or just close the settings."]],
                     [1.0 * inch, CW - 2.4 * inch - 1.0 * inch])]],
             colWidths=[2.4 * inch, CW - 2.4 * inch])
menu.setStyle(TableStyle([("VALIGN", (0, 0), (-1, -1), "MIDDLE"), ("LEFTPADDING", (0, 0), (-1, -1), 0)]))
story += [P("Living on your desktop", "h2"),
          P("Bytepet has no window frame, just the device floating above your other windows, and clicks on the "
            "empty space around it go through to whatever is behind. <b>Drag</b> the shell to move it; it "
            "remembers its spot. The everyday settings are in <b>Town → SETUP</b>, and <b>MORE</b> "
            "there opens all of them. They appear on the device's own screen, in finer dots, so the rest of the device stays in "
            "sight. Use the device's buttons: <b>A</b> moves to the next row, <b>B</b> changes or opens it and "
            "<b>C</b> goes back. With the mouse, <b>scroll</b> to pick a row, <b>click</b> to change or open it "
            "and <b>right-click</b> to go back. On the keyboard, the left and right arrows step a setting either "
            "way. Left alone, a menu or the settings go back to the home screen after a minute, and a lit icon "
            "goes out after 30 seconds; games, animations and links are never cut short:"),
          menu, Spacer(1, 6), sizes, Spacer(1, 4),
          tip("On Windows and macOS, Bytepet doesn't appear in the taskbar or Dock. Click its tray icon "
              "(notification area or menu bar) to show or hide it; right-click it for Settings and Quit. Hover "
              "over it to see your pet's hearts and what it needs without opening it. Starting Bytepet again "
              "while it's running also brings it back.", "NOTE"),
          P("Linux notes", "h2"),
          B("On Wayland desktops (Fedora's and Ubuntu's default), apps can't keep themselves on top, pick where "
            "they open or let clicks through. Bytepet therefore runs through XWayland, where all three work. Set "
            "%s to run as a native Wayland window instead." % code("BYTEPET_NATIVE_WAYLAND=1")),
          B("The tray icon uses the standard that KDE, Xfce, Cinnamon and most other desktops support. Stock "
            "GNOME needs the <i>AppIndicator and KStatusNotifierItem Support</i> extension; without it Bytepet "
            "simply stays in the taskbar."),
          B("The see-through effect needs a compositing desktop, which every current desktop has.")]

# 3 life cycle
child_good, child_rough = find("child", grade="a"), find("child", grade="b")
story += section(3, "Life cycle", "Your pet ages in real time. It only evolves while awake, so a pet due to "
                                  "change overnight changes when it wakes up.")
strip = Table([[char_cell("egg"), char_cell("baby"), char_cell(child_good), char_cell(find("teen", "humor")),
                char_cell(find("adult", "humor", "a")), char_cell("senior")]], colWidths=[CW / 6] * 6)
strip.setStyle(TableStyle([("ALIGN", (0, 0), (-1, -1), "CENTER")]))
story += [strip, Spacer(1, 8),
          table([["Stage", "How long", "What happens"],
                 ["<b>Egg</b>", dur(LIFE["egg"]), "Name your pet while you wait."],
                 ["<b>Baby</b>", dur(LIFE["baby"]), "Gets hungry and unhappy very quickly. Care mistakes here "
                                                    "decide which child you get. Can't play games or link yet."],
                 ["<b>Child</b>", dur(LIFE["child"]), "Mini-games, mail and school letters begin."],
                 ["<b>Teen</b>", dur(LIFE["teen"]), "Becomes one of three teens depending on its highest skill."],
                 ["<b>Adult</b>", "from age %d" % ADULT_AGE,
                  "Job offers arrive. The matchmaker is open from age %d." % LIFE["matchmaker_age_days"]],
                 ["<b>Senior</b>", "from age %d" % LIFE["senior_age_days"],
                  "Happens if your adult never married. Seniors can still marry, but only another senior."]],
                [0.8 * inch, 1.15 * inch, CW - 1.95 * inch]),
          P("Bedtimes", "h2"),
          P("Babies don't sleep. Everyone else follows a schedule based on your computer's clock. While asleep, "
            "hunger and happiness don't drop and no poop or mail arrives."),
          table([["Stage", "Falls asleep", "Wakes up"]] +
                [[s.title(), "%d:00" % STAGES[s]["sleep"][0], "%d:00" % STAGES[s]["sleep"][1]]
                 for s in ("child", "teen", "adult", "senior")],
                [1.4 * inch, 1.4 * inch, 1.4 * inch]),
          Spacer(1, 6),
          P("If you sleep by day (night shifts, say), <b>Town → SETUP → BEDTIME</b> moves the whole schedule "
            "by up to 12 hours either way. At +8H, a child that would sleep from 20:00 to 08:00 sleeps from "
            "04:00 to 16:00 instead."),
          tip("Faster time speeds only make your pet age faster. Bedtimes always follow your real clock (moved "
              "by BEDTIME, if you set it), so switching back to 1× needs no clean-up.")]

# 4 care
story += section(4, "Daily care", "A well-cared-for pet grows into the best adults. Neglect leads to care "
                                  "mistakes, sickness and, eventually, your pet going home to its planet.")
story += [screens([("lcd_status_hunger", "Status: hunger hearts"), ("lcd_call", "Calling for attention"),
                   ("lcd_sick", "Sick, with three poops")]),
          P("Hunger and happiness", "h2"),
          P("Each has four hearts and drains over time, faster when your pet is young:"),
          table([["Stage", "Lose a hunger heart every", "Lose a happy heart every"]] +
                [[s.title(), mins(STAGES[s]["hunger"]), mins(STAGES[s]["happy"])]
                 for s in ("baby", "child", "teen", "adult", "senior")],
                [1.4 * inch, 2.2 * inch, 2.2 * inch]),
          P("Calls and care mistakes", "h2"),
          P("When either set of hearts runs empty, or your pet falls asleep with the lights on, it beeps and the "
            "<b>Call</b> icon lights up. Answer within <b>%s</b> or it counts as a <b>care mistake</b>. If you "
            "still haven't fed it, it calls again (after %s for a baby, %s otherwise)."
            % (dur(CARE["call_timeout"]), dur(CARE["recall_baby"]), dur(CARE["recall"]))),
          P("Weight, poop and sickness", "h2"),
          B("Meals add 1 g and snacks 2 g; games and work burn 1 g. A pet carrying more than %d g over its stage's "
            "normal weight for %s gets sick." % (CARE["overweight_margin"], dur(CARE["overweight_to_sick"]))),
          B("Poop left lying around causes sickness: after %s for a baby, otherwise after %s (%s if four have "
            "piled up)." % (dur(CARE["poop_to_sick_baby"]), dur(CARE["poop_to_sick"]),
                            dur(CARE["poop_to_sick_full"]))),
          B("Empty hearts left for %s also cause sickness." % dur(CARE["empty_to_sick"])),
          B("Use <b>Medicine</b> when you see the skull. A sick pet left untreated for %s (%s for a baby) goes "
            "home for good." % (dur(CARE["sick_to_gone"]), dur(CARE["sick_to_gone_baby"]))),
          P("Discipline", "h2"),
          P("The training meter has four steps. It fills when you react correctly:"),
          table([["Your pet…", "Do this", "Result"],
                 ["Calls, but its hearts are fine", "<b>SCOLD</b>", "Training +1 step"],
                 ["Refuses a meal while hungry (“NO!”)", "<b>SCOLD</b> within %s"
                  % dur(CARE["refuse_window"]), "Training +1 step"],
                 ["Dances happily with a music note", "<b>PRAISE</b> within %s" % dur(CARE["show_off_window"]),
                  "Training +1 step, happiness +1"],
                 ["Is behaving normally", "Don't scold", "Scolding for nothing costs a happy heart"],
                 ["Calls for no reason", "Don't praise", "Praising it spoils it: training goes down"]],
                [2.3 * inch, 1.5 * inch, CW - 3.8 * inch]),
          Spacer(1, 6),
          tip("A well-trained pet stops refusing meals. A training meter of at least three steps is needed for "
              "the best adult on every path."),
          screens([("lcd_sleep", "Asleep: turn the lights off!"), ("lcd_status_train", "Training meter"),
                   ("lcd_eat", "Eating a meal")])]

# 5 skills & evolution
teens = {p: find("teen", p) for p in ("humor", "gorgeous", "passion")}
story += section(5, "Skills and evolution", "Three skills decide what your pet becomes. Every game, class and "
                                            "toy pushes it toward one of three paths.")
story += [table([["Skill", "Trained by", "Path leads to"],
                 ["<b>Humor</b>", "CLIMB game, Humor teacher, JOKES",
                  "%s, then a Humor adult" % name(teens["humor"])],
                 ["<b>Gorgeous</b>", "TUG-O-WAR game, Gorgeous teacher, RIBBON",
                  "%s, then a Gorgeous adult" % name(teens["gorgeous"])],
                 ["<b>Passion</b>", "APPLES game, Passion teacher, BALL",
                  "%s, then a Passion adult" % name(teens["passion"])]],
                [1.0 * inch, 3.0 * inch, CW - 4.0 * inch]),
          P("How the next form is chosen", "h2"),
          B("<b>Baby → child:</b> 0–%d care mistakes as a baby gives %s; more gives %s."
            % (EVO["baby_good_mistakes"], name(child_good), name(child_rough))),
          B("<b>Child → teen:</b> whichever skill is highest (a tie is decided at random)."),
          B("<b>Teen → adult:</b> the highest skill picks the path, then care mistakes made <i>after</i> the "
            "baby stage pick the adult:"),
          table([["Grade", "Requirement"],
                 ["Secret", "%d+ in the path's skill and zero care mistakes" % EVO["secret_skill"]],
                 ["Best", "0–%d mistakes and a training meter of at least %d%%"
                  % (EVO["best_mistakes"], EVO["best_discipline"])],
                 ["Good", "Up to %d mistakes" % EVO["good_mistakes"]],
                 ["Fair", "Up to %d mistakes" % EVO["fair_mistakes"]],
                 ["Rough", "%d or more mistakes" % (EVO["fair_mistakes"] + 1)]],
                [1.0 * inch, CW - 1.0 * inch])]

gnames = ["Best", "Good", "Fair", "Rough"]
gallery = [[""] + [P("<b>%s</b>" % g, "capb") for g in gnames] + [P("<b>Secret</b>", "capb")]]
for path in ("humor", "gorgeous", "passion"):
    gallery.append([P("<b>%s</b>" % path.title(), "cellb")] +
                   [char_cell(find("adult", path, g)) for g in "abcd"] +
                   [char_cell(find("adult", path, "secret"))])
gt = Table(gallery, colWidths=[0.9 * inch] + [(CW - 0.9 * inch) / 5] * 5)
gt.setStyle(TableStyle([("ALIGN", (1, 0), (-1, -1), "CENTER"), ("VALIGN", (0, 0), (-1, -1), "MIDDLE"),
                        ("LINEBELOW", (0, 0), (-1, -1), 0.5, RULE), ("BACKGROUND", (5, 1), (5, -1), SOFT)]))
rare = next(c["id"] for c in CHARS.values() if c.get("grade") == "rare")
story += [CondPageBreak(5 * inch), P("The adults", "h2"), gt, Spacer(1, 8)]
others = Table([[char_cell(child_good, sub="Child, good care"), char_cell(child_rough, sub="Child, rough care"),
                 char_cell(teens["humor"], sub="Humor teen"), char_cell(teens["gorgeous"], sub="Gorgeous teen"),
                 char_cell(teens["passion"], sub="Passion teen"), char_cell(rare, sub="Rare, see sec. 10")]],
               colWidths=[CW / 6] * 6)
others.setStyle(TableStyle([("ALIGN", (0, 0), (-1, -1), "CENTER"), ("VALIGN", (0, 0), (-1, -1), "TOP")]))
story += [others]

# 6 games
story += section(6, "Mini-games", "Open the <b>Game</b> icon (children and older). Press B to start, or wait; "
                                  "C on the title screen backs out.")
story += [P("Winning a game gives a happy heart and a bigger skill boost (up to +15). Losing still earns a little. "
            "Every game burns 1 g of weight.")]
g_rows = [["", "Game", "How to play", "Reward"],
          [img("lcd_game_climb", 1.35 * inch), "<b>CLIMB</b>",
           "Rocks fall down two vines. Press A or C to jump between them. 30 seconds, three lives.",
           "Humor. Win: dodge 12+"],
          [img("lcd_game_tug", 1.35 * inch), "<b>TUG-O-WAR</b>",
           "Mash B to drag the flag toward your side before the rival pulls it away. 12 seconds.",
           "Gorgeous. Win: score 11+"],
          [img("lcd_game_apples", 1.35 * inch), "<b>APPLES</b>",
           "Move the basket with A and C (hold to glide). Catch apples; a bomb costs 2 points. 25 seconds.",
           "Passion. Win: 10+"],
          [img("lcd_game_manhole", 1.35 * inch), "<b>MANHOLE</b>",
           "A, B and C cover the three manholes for a moment. Cover each one as a walker steps over it. Three "
           "falls ends the game.", "10 GP per walker. Win: 8+"]]
story += [table(g_rows, [1.5 * inch, 0.95 * inch, CW - 3.7 * inch, 1.25 * inch])]

# 7 mail school jobs
story += section(7, "Mail, school and jobs", "Every few hours (while awake) the mail arrives. The Town icon "
                                             "blinks when you have letters; read them in Town → MAIL.")
story += [table([["Letter", "What it does"],
                 ["Gift", "%d–%d GP" % (min(MAIL["gp_amounts"]), max(MAIL["gp_amounts"]))],
                 ["Card", "+1 hunger and +1 happy heart"],
                 ["Snake in a box", "A prank: −1 happy heart"],
                 ["Parcel", "A free item (%s)" % ", ".join(MAIL["items"])],
                 ["School invite", "Children and teens, at most once a day. Expires after %s."
                  % dur(CARE["school_expires"])],
                 ["Job offer", "Adults without a job, once a skill reaches 70% of a job's requirement."],
                 ["Love letter", "A hint that the matchmaker is waiting."]],
                [1.4 * inch, CW - 1.4 * inch]),
          P("Up to %d letters are kept. Mail stops while your pet sleeps." % CARE["mail_max_waiting"]),
          P("School", "h2"),
          P("Accept the invite, choose a teacher (Humor, Gorgeous or Passion) and answer five quick sums: "
            "<b>A</b> picks the number on the left, <b>C</b> the one on the right, within 6 seconds each. You get "
            "+4 in that skill per right answer, +6 bonus for a perfect class, and a happy heart for 3 or more.")]
how = {"office": "Press the button under the desk where the paper lands. 3 misses ends it.",
       "science": "Watch the flasks light up, then repeat the order. Up to 6 rounds.",
       "chef": "Copy the 3-ingredient order (bun = A, meat = B, leaf = C). 30 seconds.",
       "fire": "A and C move the hose, B sprays. Don't let any fire reach the top. 3 misses ends it.",
       "rock": "Notes scroll in three rows. Press A, B or C (top, middle, bottom) as a note hits the line. "
               "40 seconds."}
j_rows = [["Job", "Needs", "Base pay", "Work game"]]
for j in JOBS:
    j_rows.append(["<b>%s</b>" % j["title"], "%s %d" % (j["skill"].title(), j["need"]), "%d GP" % j["pay"],
                   how[j["game"]]])
story += [P("Jobs", "h2"),
          P("Accept a job offer to sit a three-question interview. You're hired if you answer at least two "
            "correctly <i>and</i> meet the skill requirement. Then <b>Game → WORK</b> plays your job's game "
            "once every %s. Pay is 40–120%% of base depending on your score, plus +3 to the job's skill."
            % dur(CARE["work_cooldown"])),
          table(j_rows, [1.05 * inch, 0.95 * inch, 0.75 * inch, CW - 2.75 * inch]),
          Spacer(1, 6),
          screens([("lcd_game_office", "Office"), ("lcd_game_science", "Scientist"), ("lcd_game_chef", "Chef"),
                   ("lcd_game_fire", "Firefighter"), ("lcd_game_rock", "Rock star"),
                   ("lcd_game_quiz", "School / interview quiz")])]


# 8 town
def fx_text(fx):
    parts = []
    for k, v in fx.items():
        if k == "cure":
            parts.append("cures sickness instantly")
        elif k == "weight":
            parts.append("+%d g" % v)
        elif k in ("happy", "hunger"):
            parts.append("+%d %s heart%s" % (v, k, "s" if v > 1 else ""))
        else:
            parts.append("+%d %s" % (v, k.title()))
    return ", ".join(parts)


story += section(8, "Town", "The house icon. Your Gotchi Points (GP), items, friends and album all carry over "
                            "from one generation to the next.")
story += [screens([("lcd_town", "The Town menu"), ("lcd_clock", "B on the main screen shows the clock"),
                   ("lcd_foodmenu", "The food menu")]),
          P("Shop", "h2"),
          P("You start with %d GP. Earn more from MANHOLE, mail and work. Bought items go to Town → ITEMS "
            "(food can also be eaten from Food → ITEMS)." % LIFE["start_gp"]),
          table([["Item", "Price", "Effect"]] +
                [["<b>%s</b>" % i["id"].title(), "%d GP" % i["price"], fx_text(i["effects"])] for i in ITEMS],
                [1.2 * inch, 0.9 * inch, CW - 2.1 * inch]),
          P("Other Town options", "h2"),
          table([["Option", "What it does"],
                 ["MAIL", "Read the oldest letter. The label shows how many are waiting."],
                 ["ITEMS", "Use anything you've bought or received."],
                 ["PARTNER", "The matchmaker (section 10)."],
                 ["FRIENDS", "Everyone you've linked with and your friendship level."],
                 ["ALBUM", "Every past pet: its form, generation, age, job, partner and how its story ended."],
                 ["SETUP", "SPEED (1×, 10×, 60×, 600×), SOUND (OFF or a volume), SHELL (steps through the "
                           "twelve designs), "
                           "WHEN OFF (RUNS or PAUSE: whether time passes while Bytepet is closed), BEDTIME (moves "
                           "your pet's sleep up to 12 hours later or earlier, for night owls), MORE (opens the "
                           "full settings) and NEW EGG (start over)."]],
                [1.1 * inch, CW - 1.1 * inch]),
          Spacer(1, 8)]
SHELLS = ("berry", "lime", "ocean", "grape", "zebra", "flame",
          "donut", "camo", "cosmic", "sweet", "mint", "retro")
shell_rows = []
for k in range(0, len(SHELLS), 6):
    row = SHELLS[k:k + 6]
    shell_rows += [[img("shell_" + n, 0.95 * inch) for n in row], [P(n.title(), "cap") for n in row]]
shell_row = Table(shell_rows, colWidths=[CW / 6] * 6)
shell_row.setStyle(TableStyle([("ALIGN", (0, 0), (-1, -1), "CENTER"), ("LEFTPADDING", (0, 0), (-1, -1), 0),
                               ("RIGHTPADDING", (0, 0), (-1, -1), 0)]))
story += [KeepTogether([P("Shell designs", "h2"), shell_row])]
custom = table([["Page", "Settings"],
                ["BODY", "COLOR (twenty colours), PATTERN (Plain, Zebra, Flame, Candy, Camo, Dots, Stripe or "
                         "Hearts), ACCENT (the pattern's colour; AUTO keeps its own) and GLOSS (the shine)."],
                ["BUTTONS", "COLOR (AUTO matches the body), SHAPE (Round, Square, Gem, Heart or Star) and "
                            "LETTERS (the A, B and C under them)."],
                ["ANTENNA", "SHAPE (Star, Heart, Moon, Bolt, Ball or None), COLOR and RING (the key ring on top)."],
                ["SCREEN", "FACE (the panel around the screen), EDGE (its rim), FRAME (the border right around the "
                           "screen), CORNERS (Round, Soft or Square) and LCD (Classic, Gray, Blue, Amber or Pink)."],
                ["ICONS", "ICONS (the menu icons), SELECT (the selected icon), ALERT (the attention icon, lit "
                          "whenever your pet needs something) and NAME (the BYTEPET text). AUTO keeps the usual "
                          "colours."],
                ["THEMES", "Up to 9 designs of your own: SAVE the one you're wearing, then pick a THEME to WEAR "
                           "it again or DELETE it."],
                ["RANDOM", "A surprise: every part at once. If your design isn't saved, it asks first."]],
               [0.9 * inch, CW - 0.9 * inch])
story += [KeepTogether([
    P("Make it your own", "h2"),
    P("Choose <b>SHELL</b> in the settings (Town → SETUP → MORE) for a page of its own. "
      "<b>PRESET</b> steps through the designs above; the five pages below it change one part of the device "
      "each, and the rest stays as it is. Scroll to a setting and click to step to its next option, or use the "
      "<b>left</b> and <b>right</b> arrow keys to go either way. The settings stay on the screen, so you "
      "see each part change as you go. Once you've changed anything, the shell reads <b>CUSTOM</b>. "
      "Right-click or <b>C</b> goes back a page."),
    Spacer(1, 4), img("customs", CW),
    P("Six designs of your own. Between them they use every antenna (including none), every button shape, all "
      "three corner styles and all five LCD colours.", "cap")]),
    # free to run on to the next page (its header repeats), so it never leaves a gap behind
    Spacer(1, 6), custom]

# 9 linking
soulmate = max(FRIENDS, key=lambda f: f["links"])
story += section(9, "Linking two Bytepets", "The original handhelds talked by infrared. Bytepet uses your home "
                                            "network instead. Both computers must be on the same network.")
story += [P("Connecting", "h2"),
          B("On one computer choose <b>Link → HOST</b>. It shows WAITING and its IP address."),
          B("On the other choose <b>Link → JOIN</b>. It searches for a few seconds; pick the host's pet name "
            "and press B."),
          B("If nothing is found, choose <b>Link → IP</b> instead, type the host's address (with its port "
            "if the host shows one, like %s) and press Enter." % code("192.168.1.20:41980")),
          B("Both screens now show the same 4-digit <b>CODE</b>. If they match, the <b>host presses B</b> to "
            "accept (C refuses); the other screen waits until then. A stranger's device on the same network "
            "can't link without the host agreeing."),
          B("Once paired, you take turns, the <b>host</b> first: whoever's turn it is chooses what to do, and "
            "when it's done the turn passes over. Either side can leave with <b>C</b>; a wedding ends the link."),
          screens([("lcd_link_host", "Hosting: waiting with its address"),
                   ("lcd_link_code", "The host checks the pairing code"),
                   ("lcd_link_visit", "A visit")]),
          table([["Choice", "What happens"],
                 ["<b>VISIT</b>", "The pets meet on screen. Friendship +1."],
                 ["<b>GAME</b>", "A random versus game. Both players get the identical round and the higher score "
                                 "wins a happy heart. Friendship +1."],
                 ["<b>GIFT</b>", "Send any item, or 100 GP. Friendship +1."],
                 ["<b>MARRY</b>", "Propose. Needs %s friendship and two eligible pets (section 10)."
                  % soulmate["name"].title()]],
                [1.0 * inch, CW - 1.0 * inch]),
          P("Versus games", "h2"),
          table([["", "Game", "How to play"],
                 [img("lcd_game_eats", 1.3 * inch), "<b>EATS</b>", "Mash B for 6 seconds. Most bites wins."],
                 [img("lcd_game_flags", 1.3 * inch), "<b>FLAGS</b>",
                  "12 arrows. Press A for left, B for up, C for right before time runs out."],
                 [img("lcd_game_bump", 1.3 * inch), "<b>BUMP</b>",
                  "A bar sweeps back and forth. Press B when it's inside the box. Five tries; closer scores more."]],
                [1.45 * inch, 0.8 * inch, CW - 2.25 * inch]),
          P("Friendship levels", "h2"),
          P("Every visit, versus game and gift counts as a link-up, so one link session can count several "
            "times."),
          table([["Link-ups", "Level"]] + [[str(f["links"]), f["name"].title()] for f in reversed(FRIENDS)],
                [1.2 * inch, 2.2 * inch]),
          Spacer(1, 6),
          tip("The first time you HOST, Windows or macOS asks whether to allow network access. Allow it on private "
              "networks. Bytepet uses TCP port 41975 (or the next free one) and UDP port 41976. To try linking on "
              "one computer, run two copies with separate saves: %s and %s."
              % (code("bytepet --save a.json"), code("bytepet --save b.json")), "NETWORK")]

# 10 marriage
story += section(10, "Marriage and generations", "Marrying is how your family line continues. The parent moves "
                                                 "on and its egg becomes your next pet.")
story += [screens([("lcd_wedding", "The wedding"), ("lcd_evolve", "Growing up"),
                   ("lcd_gone", "A neglected pet goes home")]),
          P("Two ways to find a partner", "h2"),
          B("<b>Matchmaker</b> (Town → PARTNER): available once an adult reaches age %d, or any time for a "
            "senior. She introduces three candidates; press A to browse and B to choose."
            % LIFE["matchmaker_age_days"]),
          B("<b>Link</b>: two adults (or two seniors) who have reached %s friendship (%d link-ups) can propose to "
            "each other. If accepted, <i>both</i> players get an egg." % (soulmate["name"].title(),
                                                                          soulmate["links"])),
          P("What carries over", "h2"),
          B("The new egg is generation +1 and starts with %d%% of its parent's skills."
            % round(CARE["legacy_skill_share"] * 100)),
          B("GP, items, friends and the album all stay. The parent is recorded in the album."),
          B("Two seniors who marry produce a <b>rare egg</b>. Raised to adulthood, it always becomes <b>%s</b>, "
            "regardless of path." % name(rare)),
          P("Going home", "h2"),
          P("A pet that stays sick too long leaves on a little spaceship. It's saved in the album, and pressing B "
            "starts a fresh egg, but the generation count goes back to 1.")]

# 11 reference
story += section(11, "Quick reference")
story += [table([["Situation", "What to do"],
                 ["Call icon lit, hearts empty", "Feed (meal or snack) within %s" % dur(CARE["call_timeout"])],
                 ["Call icon lit, hearts fine", "Scold"],
                 ["Pet asleep, call icon lit", "Turn the lights off"],
                 ["Pet refuses food", "Scold"],
                 ["Pet dancing with a note", "Praise"],
                 ["Skull on screen", "Medicine (maybe twice), or a MEDKIT"],
                 ["Poop on screen", "Toilet"],
                 ["Town icon blinking", "Read your mail"],
                 ["“DAY OFF” when working", "Your pet can work once every %s" % dur(CARE["work_cooldown"])],
                 ["Bytepet disappeared", "Click its tray / menu-bar icon, or start Bytepet again"]],
                [2.4 * inch, CW - 2.4 * inch]),
          P("Time while away", "h2"),
          P("When you reopen Bytepet it simulates the time you were gone (up to 14 days) and greets you with how "
            "long you were away. If you'd rather your pet freeze while the app is closed, set Town → SETUP "
            "→ WHEN OFF to PAUSE. The same applies while your computer sleeps."),
          P("Save files", "h2"),
          table([["System", "Location"],
                 ["Windows", code("%APPDATA%\\Bytepet\\save.json")],
                 ["macOS", code("~/Library/Application Support/Bytepet/save.json")],
                 ["Linux", code("~/.local/share/bytepet/save.json")]],
                [1.1 * inch, CW - 1.1 * inch]),
          P("The file also remembers the widget's position, size and settings. The game saves every 20 seconds and "
            "when you close it. Each save is flushed to disk before it replaces the last one, so neither a crash "
            "nor a power cut can leave it half-written, and the save before it is kept as %s. If a save is ever "
            "damaged, it's kept as %s and Bytepet carries on from the backup (or, without one, a new egg "
            "starts); you get a notification saying so." % (code("save.json.bak"), code("save.json.corrupt"))),
          P("Command-line options", "h2"),
          table([["Option", "Meaning"],
                 [code("--save FILE"), "Use a different save file (one running copy per file)"],
                 [code("--speed 1|10|60|600"), "Start at a time speed"],
                 [code("--new"), "Ignore the existing save and start a new egg"],
                 [code("--windowed"), "A normal window with a title bar instead of the floating widget"],
                 [code("--export-content DIR"), "Write the built-in content files to DIR, for modding"],
                 [code("--screenshot FILE"), "Draw the device to a PNG and exit"],
                 [code("--paths"), "Show where the save file and mod folder are"]],
                [2.0 * inch, CW - 2.0 * inch]),
          P("Customizing", "h2"),
          P("Bytepet's content is plain text you can edit. Run %s to get the three files, edit them, and "
            "restart. Each file in that %s folder (next to your save) replaces the built-in version; delete it to "
            "go back." % (code("bytepet --export-content &lt;save folder&gt;/content"), code("content"))),
          table([["File", "What's in it"],
                 [code("characters.toml"), "Every character: name, stage, path, grade and its six 16×16 "
                                           "frames, drawn with %s and %s." % (code("#"), code("."))],
                 [code("props.toml"), "Food, poop, the skull, game pieces and the ten icons."],
                 [code("game.toml"), "Timings, care rules, evolution thresholds, the shop, jobs, mail and "
                                     "friendship levels."]],
                [1.6 * inch, CW - 1.6 * inch]),
          Spacer(1, 6),
          tip("A file with a mistake in it is ignored, the built-in content is used, and a notification says "
              "what's wrong, so a typo can't break your pet.")]


def keep_blocks(items):
    """Glue each heading to its intro text and first table/figure, and keep tables on one page."""
    out = []
    i = 0
    while i < len(items):
        it = items[i]
        if isinstance(it, Paragraph) and it.style.name in ("num", "h2"):
            group = [it]
            j = i + 1
            while j < len(items) and len(group) < 6:
                nxt = items[j]
                if isinstance(nxt, Paragraph) and nxt.style.name == "num":
                    break
                if isinstance(nxt, Paragraph) and nxt.style.name == "h2" and it.style.name == "h2":
                    break
                group.append(nxt)
                j += 1
                if isinstance(nxt, (Table, Image)):
                    break
            out.append(KeepTogether(group))
            i = j
            continue
        out.append(KeepTogether([it]) if isinstance(it, Table) else it)
        i += 1
    return out


# ------------------------------------------------------------------ quick reference card
# Two letter pages in two columns: everything a player looks up mid-game, nothing they read once.
QM = 0.5 * inch                   # page margin
QG = 0.28 * inch                  # gap between the columns
QW = (PW - 2 * QM - QG) / 2       # column width
QTOP = 0.62 * inch                # header band

qs = {
    "h": ParagraphStyle("qh", fontName="Helvetica-Bold", fontSize=9.5, leading=11.5, textColor=PINK, spaceBefore=8,
                        spaceAfter=3),
    "body": ParagraphStyle("qbody", fontName="Helvetica", fontSize=8.4, leading=11, textColor=INK, spaceAfter=3),
    "cell": ParagraphStyle("qcell", fontName="Helvetica", fontSize=8, leading=10, textColor=INK),
    "th": ParagraphStyle("qth", fontName="Helvetica-Bold", fontSize=7.6, leading=9.5, textColor=colors.white),
    "bullet": ParagraphStyle("qbullet", fontName="Helvetica", fontSize=8.4, leading=11, textColor=INK, leftIndent=10,
                             bulletIndent=1, spaceAfter=1.5),
}


def qtable(rows, widths, header=True):
    """A compact table sized to a column (widths are fractions of it)."""
    data = [[Paragraph(c, qs["th"] if header and i == 0 else qs["cell"]) if isinstance(c, str) else c for c in row]
            for i, row in enumerate(rows)]
    t = Table(data, colWidths=[w * QW for w in widths])
    st = [("VALIGN", (0, 0), (-1, -1), "MIDDLE"), ("LINEBELOW", (0, 0), (-1, -1), 0.4, RULE),
          ("TOPPADDING", (0, 0), (-1, -1), 2.6), ("BOTTOMPADDING", (0, 0), (-1, -1), 2.6),
          ("LEFTPADDING", (0, 0), (-1, -1), 4), ("RIGHTPADDING", (0, 0), (-1, -1), 4)]
    if header:
        st.append(("BACKGROUND", (0, 0), (-1, 0), INK))
    for i in range(1 if header else 0, len(data)):
        if i % 2 == 0:
            st.append(("BACKGROUND", (0, i), (-1, i), colors.HexColor("#F9FAFB")))
    t.setStyle(TableStyle(st))
    return t


def block(title, *items):
    """A heading kept with what follows it."""
    return KeepTogether([Paragraph(title.upper(), qs["h"])] + list(items))


def qp(text):
    return Paragraph(text, qs["body"])


def qb(text):
    return Paragraph(text, qs["bullet"], bulletText="•")


def quick_page(c, doc):
    c.saveState()
    c.setFillColor(DARK)
    c.rect(0, PH - QTOP, PW, QTOP, fill=1, stroke=0)
    c.setFillColor(PINK)
    c.rect(0, PH - QTOP - 3, PW, 3, fill=1, stroke=0)
    path = os.path.join(IMG, "title.png")
    title = Image(path)
    th = 0.22 * inch
    tw = th * title.imageWidth / title.imageHeight
    c.drawImage(path, QM, PH - 0.41 * inch, width=tw, height=th, mask="auto")
    c.setFillColor(colors.white)
    c.setFont("Helvetica-Bold", 11)
    c.drawString(QM + tw + 12, PH - 0.38 * inch, "Quick reference")
    c.setFillColor(colors.HexColor("#A1A6B4"))
    c.setFont("Helvetica", 8)
    c.drawRightString(PW - QM, PH - 0.38 * inch, "Version %s  ·  page %d" % (VERSION, doc.page))
    c.setFillColor(MUTED)
    c.setFont("Helvetica", 7)
    c.drawCentredString(PW / 2, 0.32 * inch, "Every rule, character and screen is in the Bytepet Player's Manual.  "
                                             "Not affiliated with Bandai or the Tamagotchi brand.")
    c.restoreState()


def quick_story():
    s = []
    s.append(block("Buttons", qtable(
        [["Button", "Keys", "Does"],
         ["<b>A</b>", "Z  A  J  1  ←", "Next icon or option"],
         ["<b>B</b>", "X  S  K  2  ↓  Enter  Space", "Choose. Home screen, nothing selected: the clock"],
         ["<b>C</b>", "C  D  L  3  →  Esc", "Back. Home screen: clear the selection, call your pet over"],
         ["<b>A + C</b>", "M", "Mute or unmute"]],
        [0.15, 0.36, 0.49]),
        Spacer(1, 3), qp("<b>F1</b> help  ·  <b>F2</b> time speed  ·  <b>F3</b> shell design. Click Bytepet first so "
                         "it has the keyboard.")))
    s.append(block("Mouse", qtable(
        [["Do this", "To"],
         ["Click A, B or C", "Press that button"],
         ["Click an icon", "Open it, even from inside another menu"],
         ["Click the screen", "B: choose"],
         ["Scroll down / up", "Next / previous option"],
         ["Right-click", "C: back, or clear the selected icon"],
         ["Drag the shell or screen", "Move Bytepet"]],
        [0.36, 0.64])))

    icons = [("Status", "Age, weight, hearts, training, skills, GP, job"),
             ("Food", "MEAL: +1 hunger. SNACK: +1 happy, more weight"),
             ("Toilet", "Flush the poop"),
             ("Game", "Mini-games, and WORK once hired"),
             ("Lights", "Turn off when your pet falls asleep"),
             ("Medicine", "Cures the skull (some need two doses)"),
             ("Discipline", "SCOLD or PRAISE"),
             ("Link", "Play with another Bytepet on your network"),
             ("Town", "Mail, shop, items, partner, friends, album, setup"),
             ("Call", "Lights up when your pet needs you")]
    s.append(block("The ten icons", qtable(
        [[img("icon_%d" % i, 0.19 * inch), "<b>%s</b>" % n, d] for i, (n, d) in enumerate(icons)],
        [0.1, 0.22, 0.68], header=False)))

    s.append(block("Skills", qtable(
        [["Skill", "Raised by"],
         ["<b>Humor</b>", "CLIMB, the Humor teacher, JOKES"],
         ["<b>Gorgeous</b>", "TUG-O-WAR, the Gorgeous teacher, RIBBON"],
         ["<b>Passion</b>", "APPLES, the Passion teacher, BALL"],
         ["<b>GP</b> (money)", "MANHOLE, mail, WORK. You start with %d GP" % LIFE["start_gp"]]],
        [0.26, 0.74])))

    s.append(block("When your pet…", qtable(
        [["You see", "Do this"],
         ["Call icon, empty hearts", "Feed it (meal or snack) within %s" % dur(CARE["call_timeout"])],
         ["Call icon, hearts are fine", "<b>Scold</b>: it's testing you"],
         ["Call icon while asleep", "Turn the lights off"],
         ["“NO!” at mealtime", "<b>Scold</b> within %s" % dur(CARE["refuse_window"])],
         ["A dance with a music note", "<b>Praise</b> within %s" % dur(CARE["show_off_window"])],
         ["A skull", "Medicine (maybe twice), or a MEDKIT"],
         ["Poop", "Toilet. Left too long, it causes sickness"],
         ["Town icon blinking", "Read your mail"],
         ["Bytepet vanished", "Click the tray / menu-bar icon, or start it again"]],
        [0.42, 0.58]),
        Spacer(1, 3),
        qp("An unanswered call becomes a <b>care mistake</b> after %s. Scolding for nothing costs a happy heart; "
           "praising a fake call lowers training." % dur(CARE["call_timeout"]))))

    def sleep(st):
        return "%d:00 – %d:00" % tuple(STAGES[st]["sleep"]) if "sleep" in STAGES[st] else "Never"

    stage_rows = [["Stage", "Lasts", "Lose a heart every", "Asleep"],
                  ["Egg", dur(LIFE["egg"]), "–", "–"]]
    for st, lasts in (("baby", dur(LIFE["baby"])), ("child", dur(LIFE["child"])), ("teen", dur(LIFE["teen"])),
                      ("adult", "from age %d" % ADULT_AGE), ("senior", "age %d, if unmarried" % LIFE["senior_age_days"])):
        stage_rows.append([st.title(), lasts, "%s hunger · %s happy" % (mins(STAGES[st]["hunger"]),
                                                                       mins(STAGES[st]["happy"])), sleep(st)])
    s.append(block("Life stages", qtable(stage_rows, [0.16, 0.25, 0.35, 0.24]),
                   Spacer(1, 3), qp("Bedtimes follow your computer's clock at every time speed; "
                                    "<b>Town → SETUP → BEDTIME</b> moves them up to 12 hours. Asleep, nothing "
                                    "drains and no poop or mail arrives.")))

    child_good, child_rough = find("child", grade="a"), find("child", grade="b")
    s.append(block("Growing up",
                   qb("<b>Baby → child:</b> 0–%d care mistakes gives %s, more gives %s."
                      % (EVO["baby_good_mistakes"], name(child_good), name(child_rough))),
                   qb("<b>Child → teen:</b> the highest skill picks the path."),
                   qb("<b>Teen → adult:</b> the highest skill again, then mistakes made after the baby stage:"),
                   Spacer(1, 2),
                   qtable([["Adult", "Needs"],
                           ["Secret", "%d+ in the path's skill, zero mistakes" % EVO["secret_skill"]],
                           ["Best", "0–%d mistakes and training at %d%%+"
                            % (EVO["best_mistakes"], EVO["best_discipline"])],
                           ["Good", "Up to %d mistakes" % EVO["good_mistakes"]],
                           ["Fair", "Up to %d mistakes" % EVO["fair_mistakes"]],
                           ["Rough", "%d or more" % (EVO["fair_mistakes"] + 1)]],
                          [0.22, 0.78])))

    s.append(block("Mini-games", qtable(
        [["Game", "Buttons", "Win"],
         ["<b>CLIMB</b>", "A / C jump between vines, dodge rocks", "Dodge 12+"],
         ["<b>TUG-O-WAR</b>", "Mash B", "Score 11+"],
         ["<b>APPLES</b>", "A / C move the basket; bombs cost 2", "Catch 10+"],
         ["<b>MANHOLE</b>", "A, B, C cover a manhole as a walker steps on it", "8+ (10 GP each)"]],
        [0.25, 0.52, 0.23]),
        Spacer(1, 3), qp("A win gives a happy heart and up to +15 skill. Every game burns 1 g.")))

    s.append(block("School and jobs",
                   qp("<b>School</b> (children and teens): accept the letter, pick a teacher, answer five sums. "
                      "<b>A</b> picks the left number, <b>C</b> the right. +4 skill per right answer."),
                   qtable([["Job", "Needs", "Base pay"]] +
                          [["<b>%s</b>" % j["title"], "%s %d" % (j["skill"].title(), j["need"]), "%d GP" % j["pay"]]
                           for j in JOBS],
                          [0.42, 0.33, 0.25]),
                   Spacer(1, 3),
                   qp("Offers come by mail. Pass the interview (2 of 3 right), then <b>Game → WORK</b> once every %s "
                      "for 40–120%% of base pay." % dur(CARE["work_cooldown"]))))

    s.append(block("Mail", qtable(
        [["Letter", "Brings"],
         ["Gift", "%d–%d GP" % (min(MAIL["gp_amounts"]), max(MAIL["gp_amounts"]))],
         ["Card", "+1 hunger and +1 happy heart"],
         ["Snake in a box", "A prank: −1 happy heart"],
         ["Parcel", "A free item"],
         ["School invite", "Children and teens, once a day. Expires after %s" % dur(CARE["school_expires"])],
         ["Job offer", "Adults, once a skill is close to a job's need"],
         ["Love letter", "The matchmaker is waiting"]],
        [0.3, 0.7]),
        Spacer(1, 3), qp("Read it in Town → MAIL. Up to %d letters wait; none arrive while your pet sleeps."
                         % CARE["mail_max_waiting"])))

    s.append(block("Shop", qtable(
        [["Item", "GP", "Does"]] + [["<b>%s</b>" % i["id"].title(), str(i["price"]), fx_text(i["effects"])]
                                    for i in ITEMS],
        [0.27, 0.13, 0.6])))

    soulmate = max(FRIENDS, key=lambda f: f["links"])
    rare = next(c["id"] for c in CHARS.values() if c.get("grade") == "rare")
    s.append(block("Linking",
                   qb("One computer picks <b>Link → HOST</b>; the other <b>Link → JOIN</b> (or <b>IP</b> to type the "
                      "address). Same network only."),
                   qb("Both screens show a 4-digit code. If it matches, the host presses <b>B</b>."),
                   qb("Take turns, host first: pick <b>VISIT</b>, <b>GAME</b> (EATS, FLAGS or BUMP), <b>GIFT</b> or <b>MARRY</b>. "
                      "Then it's the other side's turn. <b>C</b> leaves."),
                   Spacer(1, 2),
                   qtable([["Link game", "How to win"],
                           ["<b>EATS</b>", "Mash B for 6 seconds"],
                           ["<b>FLAGS</b>", "12 arrows: A left, B up, C right"],
                           ["<b>BUMP</b>", "Press B while the bar is in the box, 5 tries"]],
                          [0.25, 0.75]),
                   Spacer(1, 3),
                   qp("<b>Friendship:</b> " + "  ·  ".join("%d %s" % (f["links"], f["name"].title())
                                                            for f in FRIENDS))))

    s.append(block("Marriage",
                   qb("<b>Matchmaker:</b> Town → PARTNER, from age %d (seniors any time)."
                      % LIFE["matchmaker_age_days"]),
                   qb("<b>By link:</b> two adults, or two seniors, at %s (%d links). Both players get an egg."
                      % (soulmate["name"].title(), soulmate["links"])),
                   qb("The egg is the next generation and keeps %d%% of its parent's skills. GP, items, friends and "
                      "the album carry over." % round(CARE["legacy_skill_share"] * 100)),
                   qb("Two seniors make a rare egg, which grows into %s." % name(rare))))

    s.append(block("Settings",
                   qp("Town → SETUP → MORE, or Settings on the tray icon: <b>size</b>, <b>shell</b> (with up to 9 "
                      "saved <b>themes</b>), <b>time speed</b> (1×, 10×, 60×, 600×), <b>sound</b>, "
                      "<b>always on top</b>, <b>notifications</b>, <b>launch at login</b>, help, hide and quit."),
                   qp("Time keeps running while Bytepet is closed (up to 14 days). To pause it instead, set "
                      "<b>Town → SETUP → WHEN OFF</b> to PAUSE.")))

    s.append(block("Your save",
                   qtable([["Windows", code("%APPDATA%\\Bytepet\\save.json")],
                           ["macOS", code("~/Library/Application Support/Bytepet/save.json")],
                           ["Linux", code("~/.local/share/bytepet/save.json")]],
                          [0.22, 0.78], header=False),
                   Spacer(1, 3), qp("Saved every 20 seconds and on exit, with the previous save kept as %s. "
                                    "%s prints the exact path." % (code("save.json.bak"), code("bytepet --paths")))))
    return s


def build():
    os.makedirs(OUT_DIR, exist_ok=True)
    out = os.path.join(OUT_DIR, "Bytepet_Manual.pdf")
    doc = BaseDocTemplate(out, pagesize=letter, leftMargin=M, rightMargin=M, topMargin=0.85 * inch,
                          bottomMargin=0.8 * inch, title="Bytepet Player's Manual", author="Bytepet",
                          subject="Player's manual for Bytepet %s" % VERSION)
    frame = Frame(M, 0.8 * inch, CW, PH - 0.85 * inch - 0.8 * inch, id="f")
    doc.addPageTemplates([PageTemplate("cover", [frame], onPage=cover),
                          PageTemplate("body", [frame], onPage=normal)])
    doc.build(keep_blocks(story))
    print("wrote", out)

    out = os.path.join(OUT_DIR, "Bytepet_Quick_Reference.pdf")
    doc = BaseDocTemplate(out, pagesize=letter, title="Bytepet Quick Reference", author="Bytepet",
                          subject="Quick reference for Bytepet %s" % VERSION)
    top, bottom = PH - QTOP - 0.18 * inch, 0.5 * inch
    cols = [Frame(QM + i * (QW + QG), bottom, QW, top - bottom, id="c%d" % i, leftPadding=0, rightPadding=0,
                  topPadding=0, bottomPadding=0) for i in range(2)]
    doc.addPageTemplates([PageTemplate("quick", cols, onPage=quick_page)])
    doc.build(quick_story())
    print("wrote", out)


build()
