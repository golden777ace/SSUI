"""Бенчмарки SSUI против Tkinter (Windows).

Путь в репозитории: benchmarks/benchmark.py

Запуск из корня репозитория:
    python benchmarks/benchmark.py
    python benchmarks/benchmark.py --only anim
    python benchmarks/benchmark.py --repeat 5 --json results.json

Каждый замер выполняется в отдельном процессе.
Окна закрываются автоматически: вспомогательный процесс находит
окно по заголовку и шлёт WM_CLOSE. Результат — markdown-таблица.
"""

import argparse
import ctypes
import json
import os
import statistics
import subprocess
import sys
import tempfile
import time

WM_CLOSE = 0x0010

TESTS = [
    ("import", "Импорт библиотеки, мс", 0, 3),
    ("build", "Дерево из {n} виджетов, мс", 1000, 3),
    ("table", "Таблица 10×{n}, мс", 300, 3),
    ("signal", "{n} обновлений реактивной строки, мс", 50000, 3),
    ("canvas", "Canvas из {n} фигур, мс", 3000, 3),
    ("mem", "Память после {n} виджетов, МБ", 2000, 1),
    ("window", "Окно: от старта до показа, мс", 0, 2),
    ("anim", "Анимация 3 с при {n} виджетах: CPU, %", 400, 1),
]


def uniq_title(lib):
    return f"BENCH-{lib}-{os.getpid()}"


def spawn_closer(title, dwell):
    tmp = tempfile.mktemp(prefix="ssui_bench_")
    subprocess.Popen(
        [sys.executable, os.path.abspath(__file__),
         "--closer", title, str(dwell), tmp],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    return tmp


def closer_main(title, dwell, tmp):
    user32 = ctypes.windll.user32
    deadline = time.time() + 30.0
    hwnd = 0
    while time.time() < deadline:
        hwnd = user32.FindWindowW(None, title)
        if hwnd and user32.IsWindowVisible(hwnd):
            break
        hwnd = 0
        time.sleep(0.004)
    if not hwnd:
        return
    with open(tmp, "w") as f:
        f.write(repr(time.time()))
    time.sleep(dwell)
    user32.PostMessageW(hwnd, WM_CLOSE, 0, 0)


def rss_mb():
    class PMC(ctypes.Structure):
        _fields_ = [
            ("cb", ctypes.c_uint32),
            ("PageFaultCount", ctypes.c_uint32),
            ("PeakWorkingSetSize", ctypes.c_size_t),
            ("WorkingSetSize", ctypes.c_size_t),
            ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
            ("QuotaPagedPoolUsage", ctypes.c_size_t),
            ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
            ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
            ("PagefileUsage", ctypes.c_size_t),
            ("PeakPagefileUsage", ctypes.c_size_t),
        ]

    pmc = PMC()
    pmc.cb = ctypes.sizeof(PMC)
    handle = ctypes.windll.kernel32.GetCurrentProcess()
    ctypes.windll.psapi.GetProcessMemoryInfo(handle, ctypes.byref(pmc), pmc.cb)
    return pmc.WorkingSetSize / (1024.0 * 1024.0)


def bench_shapes(n):
    out = []
    for i in range(n):
        x = float(i % 60) * 18.0
        y = float(i // 60) * 12.0
        k = i % 3
        if k == 0:
            out.append(("rect", [x, y, 15.0, 9.0, 3.0, 0.0], "#3B82F6", ""))
        elif k == 1:
            out.append(("circle", [x + 8.0, y + 5.0, 5.0, 0.0], "#22C55E", ""))
        else:
            out.append(("line", [x, y, x + 15.0, y + 9.0, 1.0], "#EF4444", ""))
    return out


def w_import(lib, n):
    t0 = time.perf_counter()
    if lib == "ssui":
        import ssui
    else:
        import tkinter
    return {"value": (time.perf_counter() - t0) * 1000.0}


def w_build(lib, n):
    if lib == "ssui":
        import ssui
        win = ssui.W(uniq_title(lib), 1200, 800, thm="drk")
        t0 = time.perf_counter()
        with win.bx(pd=8.0, gp=2.0):
            for i in range(n // 2):
                win.lb(f"Метка {i}", h=24.0)
                win.bt(f"Кнопка {i}", h=30.0)
        return {"value": (time.perf_counter() - t0) * 1000.0}
    import tkinter as tk
    root = tk.Tk()
    root.title(uniq_title(lib))
    t0 = time.perf_counter()
    frame = tk.Frame(root)
    frame.pack()
    for i in range(n // 2):
        tk.Label(frame, text=f"Метка {i}").pack()
        tk.Button(frame, text=f"Кнопка {i}").pack()
    root.update_idletasks()
    dt = (time.perf_counter() - t0) * 1000.0
    root.destroy()
    return {"value": dt}


def w_table(lib, n):
    cols = [f"К{j}" for j in range(10)]
    rows = [[f"r{i}c{j}" for j in range(10)] for i in range(n)]
    if lib == "ssui":
        import ssui
        win = ssui.W(uniq_title(lib), 1200, 800, thm="drk")
        t0 = time.perf_counter()
        with win.bx(pd=8.0, gp=2.0):
            win.tbl(cols, rows, hl=1.0, vl=1.0, h=700.0)
        return {"value": (time.perf_counter() - t0) * 1000.0}
    import tkinter as tk
    from tkinter import ttk
    root = tk.Tk()
    root.title(uniq_title(lib))
    t0 = time.perf_counter()
    ids = [str(j) for j in range(10)]
    tree = ttk.Treeview(root, columns=ids, show="headings")
    for j, c in enumerate(cols):
        tree.heading(ids[j], text=c)
    for r in rows:
        tree.insert("", "end", values=r)
    tree.pack()
    root.update_idletasks()
    dt = (time.perf_counter() - t0) * 1000.0
    root.destroy()
    return {"value": dt}


def w_signal(lib, n):
    if lib == "ssui":
        import ssui
        sig = ssui.sgnl(0)
        win = ssui.W(uniq_title(lib), 640, 480, thm="drk")
        with win.bx(pd=8.0, gp=2.0):
            win.lb(bind=lambda: f"v={sig()}", h=24.0)
        t0 = time.perf_counter()
        for i in range(n):
            sig.st(i)
        return {"value": (time.perf_counter() - t0) * 1000.0}
    import tkinter as tk
    root = tk.Tk()
    root.title(uniq_title(lib))
    var = tk.StringVar(master=root)
    tk.Label(root, textvariable=var).pack()
    root.update_idletasks()
    t0 = time.perf_counter()
    for i in range(n):
        var.set(f"v={i}")
    dt = (time.perf_counter() - t0) * 1000.0
    root.destroy()
    return {"value": dt}


def w_canvas(lib, n):
    if lib == "ssui":
        import ssui
        win = ssui.W(uniq_title(lib), 1200, 800, thm="drk")
        shapes = bench_shapes(n)
        t0 = time.perf_counter()
        with win.bx(pd=8.0, gp=2.0):
            win.cv(shapes, h=700.0)
        return {"value": (time.perf_counter() - t0) * 1000.0}
    import tkinter as tk
    root = tk.Tk()
    root.title(uniq_title(lib))
    cv = tk.Canvas(root, width=1100, height=700)
    cv.pack()
    items = []
    for i in range(n):
        x = (i % 60) * 18
        y = (i // 60) * 12
        items.append((i % 3, x, y))
    t0 = time.perf_counter()
    for k, x, y in items:
        if k == 0:
            cv.create_rectangle(x, y, x + 15, y + 9, fill="#3B82F6", width=0)
        elif k == 1:
            cv.create_oval(x + 3, y, x + 13, y + 10, fill="#22C55E", width=0)
        else:
            cv.create_line(x, y, x + 15, y + 9, fill="#EF4444")
    root.update_idletasks()
    dt = (time.perf_counter() - t0) * 1000.0
    root.destroy()
    return {"value": dt}


def w_mem(lib, n):
    if lib == "ssui":
        import ssui
        win = ssui.W(uniq_title(lib), 1200, 800, thm="drk")
        with win.bx(pd=8.0, gp=2.0):
            for i in range(n // 2):
                win.lb(f"Метка {i}", h=24.0)
                win.bt(f"Кнопка {i}", h=30.0)
        return {"value": rss_mb()}
    import tkinter as tk
    root = tk.Tk()
    root.title(uniq_title(lib))
    frame = tk.Frame(root)
    frame.pack()
    for i in range(n // 2):
        tk.Label(frame, text=f"Метка {i}").pack()
        tk.Button(frame, text=f"Кнопка {i}").pack()
    root.update_idletasks()
    value = rss_mb()
    root.destroy()
    return {"value": value}


def w_window(lib, n):
    title = uniq_title(lib)
    tmp = spawn_closer(title, 0.6)
    if lib == "ssui":
        import ssui
        win = ssui.W(title, 900, 600, thm="drk")
        with win.bx(pd=10.0, gp=6.0):
            for i in range(10):
                win.bt(f"Кнопка {i}", h=36.0)
        t0 = time.time()
        win.go()
    else:
        import tkinter as tk
        root = tk.Tk()
        root.title(title)
        for i in range(10):
            tk.Button(root, text=f"Кнопка {i}").pack()
        t0 = time.time()
        root.mainloop()
    with open(tmp) as f:
        shown = float(f.read())
    try:
        os.unlink(tmp)
    except OSError:
        pass
    return {"value": (shown - t0) * 1000.0}


def w_anim(lib, n):
    title = uniq_title(lib)
    if lib == "ssui":
        import ssui
        frames = [0]
        t = ssui.sgnl(0.0)
        win = ssui.W(title, 1100, 760, thm="drk")
        fx = win.fx()

        def scene():
            frames[0] += 1
            x = 30.0 + t() * 700.0
            out = [("rect", [10.0, 10.0, 1000.0, 320.0, 10.0, 2.0], "#4B5563", "")]
            for i in range(30):
                out.append(("circle",
                            [x + i * 6.0, 40.0 + (i % 10) * 26.0, 8.0, 0.0],
                            "#22C55E", ""))
                out.append(("line",
                            [20.0, 24.0 + i * 9.0, x + 260.0, 30.0 + i * 9.0, 1.0],
                            "#3B82F6", ""))
            return out

        with win.bx(pd=8.0, gp=4.0):
            win.cv(bind=scene, h=340.0)
            win.lb(bind=lambda: f"t = {t():.2f}", h=24.0)
            with win.scr(h=300.0):
                for i in range(n):
                    win.bt(f"Кнопка {i}", h=30.0)
        fx(t, 1.0, dur=3.0)
        tmp = spawn_closer(title, 3.4)
        c0 = time.process_time()
        w0 = time.perf_counter()
        win.go()
        cpu = time.process_time() - c0
        wall = max(time.perf_counter() - w0, 1e-6)
        try:
            os.unlink(tmp)
        except OSError:
            pass
        return {"value": 100.0 * cpu / wall,
                "frames": frames[0], "wall_s": round(wall, 2)}
    import tkinter as tk
    frames = [0]
    root = tk.Tk()
    root.title(title)
    cv = tk.Canvas(root, width=1000, height=340)
    cv.pack()
    items = []
    for i in range(30):
        y = 40 + (i % 10) * 26
        items.append(cv.create_oval(30, y, 46, y + 16, fill="#22C55E", width=0))
        items.append(cv.create_line(20, 24 + i * 9, 290, 30 + i * 9, fill="#3B82F6"))
    lbl = tk.Label(root, text="t = 0.00")
    lbl.pack()
    box = tk.Frame(root)
    box.pack()
    for i in range(n):
        tk.Button(box, text=f"Кнопка {i}").grid(row=i // 8, column=i % 8)
    start = time.perf_counter()

    def step():
        frames[0] += 1
        tt = min((time.perf_counter() - start) / 3.0, 1.0)
        x = 30 + tt * 700
        for i in range(30):
            y = 40 + (i % 10) * 26
            cv.coords(items[2 * i], x + i * 6, y, x + i * 6 + 16, y + 16)
            cv.coords(items[2 * i + 1], 20, 24 + i * 9, x + 260, 30 + i * 9)
        lbl.config(text=f"t = {tt:.2f}")
        root.after(16, step)

    root.after(16, step)
    root.after(3400, root.destroy)
    c0 = time.process_time()
    w0 = time.perf_counter()
    root.mainloop()
    cpu = time.process_time() - c0
    wall = max(time.perf_counter() - w0, 1e-6)
    return {"value": 100.0 * cpu / wall,
            "frames": frames[0], "wall_s": round(wall, 2)}


WORKERS = {
    "import": w_import,
    "build": w_build,
    "table": w_table,
    "signal": w_signal,
    "canvas": w_canvas,
    "mem": w_mem,
    "window": w_window,
    "anim": w_anim,
}


def worker_main(lib, test, n):
    try:
        res = WORKERS[test](lib, n)
        res["ok"] = True
    except Exception as e:
        res = {"ok": False, "error": f"{type(e).__name__}: {e}"}
    print(json.dumps(res, ensure_ascii=False))


def run_worker(lib, test, n):
    env = dict(os.environ, PYTHONIOENCODING="utf-8")
    try:
        p = subprocess.run(
            [sys.executable, os.path.abspath(__file__),
             "--worker", lib, test, str(n)],
            capture_output=True, text=True, encoding="utf-8",
            timeout=180, env=env,
        )
    except subprocess.TimeoutExpired:
        return {"ok": False, "error": "таймаут 180 с"}
    lines = [s for s in (p.stdout or "").strip().splitlines() if s.strip()]
    if not lines:
        return {"ok": False, "error": (p.stderr or "пустой вывод").strip()[-300:]}
    try:
        return json.loads(lines[-1])
    except json.JSONDecodeError:
        return {"ok": False, "error": lines[-1][:300]}


def fmt(v):
    if v is None:
        return "—"
    return f"{v:.1f}"


def cell_text(cell):
    text = fmt(cell["value"])
    if cell["extra"].get("frames") is not None:
        text += f" · {cell['extra']['frames']} кадров"
    return text


def render(rows):
    out = [
        "| Дисциплина | Tkinter | SSUI | Выигрыш SSUI |",
        "|---|---|---|---|",
    ]
    for label, cell in rows:
        tk_v = cell["tkinter"]["value"]
        ss_v = cell["ssui"]["value"]
        ratio = "—"
        if tk_v is not None and ss_v not in (None, 0):
            ratio = f"×{tk_v / ss_v:.1f}"
        out.append(f"| {label} | {cell_text(cell['tkinter'])} "
                   f"| {cell_text(cell['ssui'])} | {ratio} |")
        for lib in ("tkinter", "ssui"):
            if cell[lib]["error"]:
                out.append(f"| ⚠ {lib} | {cell[lib]['error']} | | |")
    out.append("")
    out.append("Все дисциплины: меньше — лучше; «кадры» — больше — лучше.")
    return "\n".join(out)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--only", help="запустить одну дисциплину")
    ap.add_argument("--repeat", type=int, default=0, help="повторов на замер")
    ap.add_argument("--json", help="сохранить результаты в JSON")
    ap.add_argument("--worker", nargs=3, help=argparse.SUPPRESS)
    ap.add_argument("--closer", nargs=3, help=argparse.SUPPRESS)
    args = ap.parse_args()

    if args.closer:
        closer_main(args.closer[0], float(args.closer[1]), args.closer[2])
        return
    if args.worker:
        worker_main(args.worker[0], args.worker[1], int(args.worker[2]))
        return
    if sys.platform != "win32":
        sys.exit("Бенчмарки рассчитаны на Windows.")

    rows = []
    data = {}
    for test, label, n, reps in TESTS:
        if args.only and args.only != test:
            continue
        reps = args.repeat or reps
        cell = {}
        for lib in ("tkinter", "ssui"):
            values, extra, err = [], {}, None
            for _ in range(reps):
                r = run_worker(lib, test, n)
                if r.get("ok"):
                    values.append(r["value"])
                    extra = {k: v for k, v in r.items()
                             if k not in ("ok", "value")}
                else:
                    err = r.get("error")
            cell[lib] = {
                "value": statistics.median(values) if values else None,
                "extra": extra,
                "error": None if values else err,
            }
        data[test] = cell
        rows.append((label.format(n=n), cell))
        print(f"· {test}: готово", file=sys.stderr)

    print(render(rows))
    if args.json:
        with open(args.json, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)


if __name__ == "__main__":
    main()