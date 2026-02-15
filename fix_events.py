import re

with open("desktop/ui/index.html", "r") as f:
    content = f.read()
content = content.replace("await listen(", "listen(")
