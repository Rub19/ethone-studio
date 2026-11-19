#!/usr/bin/perl
# Fills empty English entries of every dictionary with the translation another app already has.
# usage: perl scripts/i18n-share.pl
use strict; use warnings; use utf8;
my @dicts = (glob('apps/*/i18n/en.tsv'), 'crates/studio-ui/i18n/en.tsv');
my %known;
for my $d (@dicts) {
  open my $in, '<:utf8', $d or die;
  while (<$in>) {
    s/\r?\n$//;
    my ($fr, $en) = split /\t/, $_, 2;
    $known{$fr} //= $en if defined $en && $en ne '' && $en ne $fr && $fr !~ /^#/;
  }
}
for my $d (@dicts) {
  open my $in, '<:utf8', $d or die; my @l = <$in>; close $in;
  my $n = 0;
  for (@l) {
    my ($fr, $en) = /^([^\t#][^\t]*)\t([^\r\n]*)/ or next;
    next unless $en eq '' && $known{$fr};
    $_ = "$fr\t$known{$fr}\n"; $n++;
  }
  open my $out, '>:utf8', $d or die; print $out @l; close $out;
  printf "%-32s %3d filled\n", $d, $n;
}
