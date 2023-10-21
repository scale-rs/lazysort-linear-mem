# Automate and keep safe

*IMPORTANT*: Last rule wins in this file.

## The last rule

*IMPORTANT*: This must be the last rule.

The .github directory is very sensitive:

- It contains settings.yml, which affects repo-wide config; writing that file is equivalent to being
  a repo admin.
- it contains the GitHub Actions workflows, which can access repo secrets.
