---
# synthetic fixture (P11), not canon: HTML in a title and a body shows as text
id: synthetic-html
kind: thing
title: "<img src=x onerror=alert(1)> <script>alert(2)</script>"
type: synthetic
---
Synthetic text with <img src=x onerror=alert(3)> and <script>alert(4)</script> inside.

<script>alert(5)</script>

<img src=x onerror=alert(6)>

A synthetic [synthetic site](https://example.invalid/x) and a synthetic image
![synthetic image](https://example.invalid/y.png), shown as text.
