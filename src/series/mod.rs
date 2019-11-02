/*!
  This module contains predefined types of series.
  The series in Plotters is actually an iterator of elements, which
  can be taken by `ChartContext::draw_series` function.

  This module defines some "iterator transformer", which transform the data
  iterator to the element iterator.

  Any type that implements iterator emitting drawable elements are acceptable series.
  So iterator combinator such as `map`, `zip`, etc can also be used.
*/

mod area_series;
mod bar_series;
mod histogram;
mod line_series;
mod point_series;

pub use area_series::AreaSeries;
pub use bar_series::BarSeries;
pub use histogram::Histogram;
pub use line_series::LineSeries;
pub use point_series::PointSeries;
