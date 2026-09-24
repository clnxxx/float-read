# 发版（维护者）

macOS + Windows 安装包由 GitHub Actions 在打 tag 时构建。

1. 确保 `main` 已推送
2. 打标签：`git tag v0.1.0 && git push origin v0.1.0`
3. 等待 Actions 工作流 `release` 完成（macOS universal dmg/app、Windows x64 setup.exe）
4. 在 Releases 页检查 **Draft** 资产后 **Publish**

手动跑：Actions → `release` → Run workflow（会尝试用 `v0.0.0-dev` 挂资产）。
