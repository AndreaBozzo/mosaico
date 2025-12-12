---
name: Bug Report
about: Create a report to help us improve Mosaico
title: "[Bug]: "
labels: ["bug", "triage"]
assignees: []
---

### Component
- [ ] **Python SDK** 
- [ ] **mosaicod**

### System information
- **OS:** [e.g. Ubuntu 22.04, macOS Sonoma]
- **Python Version:** [e.g. 3.13] - **ROS Distribution (if applicable):** [e.g. ROS 2 Humble, ROS 1 Noetic, None]
- **Installation Method:** [e.g. Poetry, Cargo, Source]

### Describe the bug
A clear and concise description of what the bug is.

### Steps to reproduce
1. Run the daemon: `mosaicod ...`
2. Run the injector/script:

```bash
# Example:
poetry run mosaico.ros_injector ./my_bag.mcap --name "Test_Seq" ...
```

### Include logs
If available include a `mosaicod` log, configure env variable `RUST_LOG=mosaico=trace` before execution.
