import { ChangeDetectionStrategy, Component } from '@angular/core';

@Component({
  selector: 'app-about',
  templateUrl: './about.html',
  styleUrl: './site-info.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class About {
  protected readonly donationImage = '/api/images/bfb4f7e519356ea59228913812641b7cd150e6fae0391d42b936e3e403c31da8.jpg';
}
