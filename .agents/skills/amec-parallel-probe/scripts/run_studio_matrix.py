#!/usr/bin/env python3
import concurrent.futures
import json
import os
import subprocess
import sys
import time

AMEC = os.environ.get("AMEC_BIN", "/home/aislopbox/AmeUI/target/debug/amec")
STUDIO = sys.argv[1] if len(sys.argv) > 1 else "/home/aislopbox/kagari-linux-check/src/ui/ame/kagari_studio.ame"

CASES = {
    "home_ja_1600": ["measure", STUDIO, "--width", "1600", "--height", "1000", "--output", "summary"],
    "home_en_1600": ["measure", STUDIO, "--width", "1600", "--height", "1000", "--bindings", "ui_locale=en", "--output", "summary"],
    "editor_ja_1600": ["measure", STUDIO, "--width", "1600", "--height", "1000", "--bindings", "home_page=false", "--bindings", "editor_page=true", "--output", "summary"],
    "settings_en_1600": ["measure", STUDIO, "--width", "1600", "--height", "1000", "--bindings", "ui_locale=en", "--bindings", "settings_page=true", "--output", "summary"],
    "export_en_1600": ["measure", STUDIO, "--width", "1600", "--height", "1000", "--bindings", "ui_locale=en", "--bindings", "home_page=false", "--bindings", "export_page=true", "--output", "summary"],
    "scale_200_1600": ["measure", STUDIO, "--width", "1600", "--height", "1000", "--scale", "2.0", "--output", "summary"],
    "scale_125_1600": ["measure", STUDIO, "--width", "1600", "--height", "1000", "--scale", "1.25", "--output", "summary"],
    "theme_light_1600": ["measure", STUDIO, "--width", "1600", "--height", "1000", "--system-theme", "light", "--output", "summary"],
    "min_size_640": ["measure", STUDIO, "--width", "640", "--height", "480", "--bindings", "narrow=true", "--output", "summary"],
    "aot_bytecode": ["build", STUDIO, "--target", "bytecode", "-o", "/tmp/kagari_probe_matrix.amec"],
}


def run_case(item):
    name, args = item
    t0 = time.time()
    proc = subprocess.run([AMEC] + args, capture_output=True, text=True, timeout=60)
    elapsed = round(time.time() - t0, 2)
    result = {"rc": proc.returncode, "sec": elapsed}
    if args[0] == "measure" and proc.returncode == 0:
        data = json.loads(proc.stdout)
        vp = data["viewports"][0]
        result["passed"] = data.get("passed")
        result["clipped_text_pixels"] = vp.get("clipped_text_pixels")
        result["layout_overflows"] = vp.get("layout_overflows")
        if vp.get("overflow_details"):
            result["top_overflows"] = [
                f"{d.get('label')}(+{d.get('overflow_x', 0)}x,{d.get('overflow_y', 0)}y)"
                for d in vp["overflow_details"][:3]
            ]
    else:
        result["msg"] = (proc.stderr or proc.stdout).strip()[:240]
    return name, result


def main():
    t_start = time.time()
    workers = os.cpu_count() or 8
    with concurrent.futures.ThreadPoolExecutor(max_workers=workers) as ex:
        results = list(ex.map(run_case, CASES.items()))
    print(f"Matrix finished in {time.time() - t_start:.2f}s using {workers} workers")
    for name, res in results:
        print(f"{name}: {json.dumps(res, ensure_ascii=False)}")


if __name__ == "__main__":
    main()
