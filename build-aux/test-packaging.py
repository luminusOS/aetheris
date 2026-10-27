#!/usr/bin/env python3
"""Static checks for the Flatpak manifest, metainfo and release workflows."""
import json
from pathlib import Path
import re
import shutil
import subprocess
import tomllib
import unittest
import xml.etree.ElementTree as ET

import yaml

ROOT = Path(__file__).resolve().parent.parent
APP_ID = "org.luminusos.Aetheris"
PINNED = re.compile(r"[^\s]+@[0-9a-f]{40}(?:\s|$)")


def workflow(name):
    text = (ROOT / ".github" / "workflows" / name).read_text()
    loaded = yaml.safe_load(text)
    loaded["on"] = loaded.pop(True, loaded.get("on"))
    return loaded, text


class FlatpakPackaging(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest = json.loads((ROOT / "build-aux" / f"{APP_ID}.json").read_text())

    def test_application_identity_and_sandbox_are_preserved(self):
        manifest = self.manifest
        self.assertEqual(manifest["id"], APP_ID)
        self.assertEqual(manifest["runtime"], "org.gnome.Platform")
        self.assertEqual(manifest["runtime-version"], "50")
        self.assertEqual(manifest["sdk"], "org.gnome.Sdk")
        self.assertEqual(manifest["command"], "aetheris")
        self.assertEqual(manifest["finish-args"], [
            "--socket=wayland", "--socket=fallback-x11", "--share=network",
            "--filesystem=~/.kube:create", "--filesystem=xdg-config/aetheris:create"])

    def test_branch_comes_from_the_release_channel_not_the_manifest(self):
        self.assertNotIn("branch", self.manifest)

    def test_appstream_catalog_data_is_composed(self):
        self.assertNotIn("appstream-compose", self.manifest)

    def test_pkg_config_search_path_covers_both_architectures(self):
        path = self.manifest["build-options"]["env"]["PKG_CONFIG_PATH"].split(":")
        self.assertIn("/app/lib/pkgconfig", path)
        self.assertIn("/app/lib/x86_64-linux-gnu/pkgconfig", path)
        self.assertIn("/app/lib/aarch64-linux-gnu/pkgconfig", path)

    def test_vte_is_built_from_a_fixed_tag(self):
        vte = next(module for module in self.manifest["modules"] if module["name"] == "vte")
        self.assertEqual(vte["sources"][0]["tag"], "0.83.91")

    def test_simdutf_is_built_before_vte_and_vte_cannot_fall_back_to_its_subproject(self):
        names = [module["name"] for module in self.manifest["modules"]]
        self.assertLess(names.index("simdutf"), names.index("vte"))
        simdutf = self.manifest["modules"][names.index("simdutf")]
        self.assertEqual(simdutf["sources"][0]["tag"], "v7.7.1")
        self.assertIn("--wrap-mode=nofallback", self.manifest["modules"][names.index("vte")]["config-opts"])

    def test_newest_metainfo_release_matches_the_workspace_version(self):
        cargo = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"]
        metainfo = ET.parse(ROOT / "data" / f"{APP_ID}.metainfo.xml").getroot()
        self.assertEqual(metainfo.find("releases/release").get("version"), cargo)

    @unittest.skipUnless(shutil.which("appstreamcli"), "appstreamcli is required")
    def test_metainfo_validates(self):
        result = subprocess.run(["appstreamcli", "validate", "--no-net",
                                 str(ROOT / "data" / f"{APP_ID}.metainfo.xml")],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_flatpak_workflow_builds_both_architectures_natively(self):
        loaded, text = workflow("release-flatpak.yml")
        include = loaded["jobs"]["flatpak"]["strategy"]["matrix"]["include"]
        self.assertEqual({(item["arch"], item["runner"]) for item in include},
                         {("x86_64", "ubuntu-24.04"), ("aarch64", "ubuntu-24.04-arm")})
        self.assertFalse(loaded["jobs"]["flatpak"]["strategy"]["fail-fast"])
        self.assertNotIn("ubuntu-latest", text)
        self.assertIn("flatpak-${{ matrix.arch }}", text)
        self.assertNotIn("retention-days", text)
        self.assertEqual(loaded["permissions"], {"contents": "read"})

    def test_flatpak_workflow_maps_release_candidates_to_testing(self):
        _, text = workflow("release-flatpak.yml")
        self.assertIn("*-rc*) BRANCH=testing", text)
        self.assertIn("*) BRANCH=stable", text)
        self.assertIn('--default-branch="${BRANCH}"', text)
        self.assertIn("aetheris-${VERSION}-linux-${ARCH}.flatpak", text)
        self.assertIn(r"(-rc\.?[1-9][0-9]*)?$", text)

    def test_flatpak_workflow_actions_are_pinned_and_secrets_are_absent(self):
        loaded, text = workflow("release-flatpak.yml")
        for use in re.findall(r"uses:\s*(\S+)", text):
            self.assertRegex(use + " ", PINNED, use)
        self.assertNotIn("secrets.", text)

    def test_build_workflow_reuses_the_release_workflow_without_publishing(self):
        loaded, text = workflow("flatpak-build.yml")
        self.assertEqual(loaded["permissions"], {"contents": "read"})
        self.assertEqual(loaded["jobs"]["flatpak"]["uses"], "./.github/workflows/release-flatpak.yml")
        self.assertIn("pull_request", loaded["on"])
        self.assertNotIn("softprops", text)
        self.assertNotIn("secrets.", text)
        for use in re.findall(r"uses:\s*(\S+)", text):
            if not use.startswith("./"):
                self.assertRegex(use + " ", PINNED, use)

    def test_release_publishes_checksums_and_every_flatpak_bundle(self):
        loaded, text = workflow("release.yml")
        release = loaded["jobs"]["release"]
        files = next(step["with"]["files"] for step in release["steps"]
                     if str(step.get("uses", "")).startswith("softprops/action-gh-release"))
        self.assertIn("dist/aetheris-*-linux-*.flatpak", files)
        self.assertIn("dist/checksums.txt", files)
        self.assertIn("pattern: flatpak-*", text)
        self.assertNotIn("needs.flatpak.outputs", text)


if __name__ == "__main__":
    unittest.main()
